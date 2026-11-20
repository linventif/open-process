//! BPMN → Petri net translation (in the spirit of Dijkman, Dumas & Ouyang, 2008).
//!
//! * every sequence flow becomes a place;
//! * a start event becomes a marked place `i` followed by a transition;
//! * an end event becomes a transition followed by a place `o`;
//! * a task / intermediate event / sub-process becomes one transition per
//!   incoming flow (implicit XOR-join), producing on all outgoing flows;
//! * XOR / event-based gateway: one silent transition per (in, out) pair;
//! * AND gateway: one silent transition consuming all inputs and producing all outputs;
//! * OR split: one silent transition per non-empty subset of outgoing flows
//!   (the OR-join is approximated as an XOR-join; a warning is emitted).
//!
//! All start events share the source place `i` and all end events the sink place `o`,
//! so the result is a workflow net for any well-formed process.

use crate::bpmn::{BpmnDiagram, FlowKind, GatewayType, Id, NodeKind};
use crate::geom::Pos;
use crate::i18n::{tr, trf};
use crate::petri::{NodeRef, PetriNet};
use std::collections::{HashMap, HashSet};

pub struct Conversion {
    pub net: PetriNet,
    pub warnings: Vec<String>,
}

/// Petri nets need more room than BPMN diagrams (a place per flow).
const SCALE: f32 = 1.6;

fn scaled(p: Pos) -> Pos {
    Pos::new(p.x * SCALE, p.y * SCALE)
}

pub fn bpmn_to_petri(d: &BpmnDiagram) -> Conversion {
    let mut net = PetriNet::new(if d.name.is_empty() {
        tr("conversion.converted_process").to_string()
    } else {
        trf("conversion.petri_name", &[&d.name])
    });
    let mut warnings = Vec::new();

    let label = |id: Id| -> String {
        d.node(id)
            .map(|n| if n.name.is_empty() { n.kind.label() } else { n.name.clone() })
            .unwrap_or_default()
    };

    // One place per sequence flow, placed at the middle of the flow.
    let mut flow_place: HashMap<Id, Id> = HashMap::new();
    for f in d.flows.iter().filter(|f| f.kind == FlowKind::Sequence) {
        let (Some(a), Some(b)) = (d.node(f.from), d.node(f.to)) else {
            continue;
        };
        let pid = net.add_place(scaled(a.pos).lerp(scaled(b.pos), 0.5), 0);
        let k = net.places.len();
        let p = net.place_mut(pid).unwrap();
        p.name = if f.name.is_empty() { format!("p{k}") } else { f.name.clone() };
        flow_place.insert(f.id, pid);
    }

    let arc = |net: &mut PetriNet, a: NodeRef, b: NodeRef| {
        let _ = net.add_arc(a, b);
    };
    let ins = |id: Id| -> Vec<Id> { d.incoming(id).filter_map(|f| flow_place.get(&f.id).copied()).collect() };
    let outs = |id: Id| -> Vec<Id> { d.outgoing(id).filter_map(|f| flow_place.get(&f.id).copied()).collect() };

    // Shared source place i and sink place o (several start / end events
    // are alternative ways to start / complete the process).
    let mut source: Option<Id> = None;
    let mut sink: Option<Id> = None;
    for n in &d.nodes {
        if !n.kind.is_flow_node() {
            continue;
        }
        let (i, o) = (ins(n.id), outs(n.id));
        let name = label(n.id);
        let new_t = |net: &mut PetriNet, name: &str, silent: bool, dy: f32| {
            let t = net.add_transition(Pos::new(n.pos.x * SCALE, n.pos.y * SCALE + dy));
            let tr = net.transition_mut(t).unwrap();
            tr.name = name.to_string();
            tr.silent = silent;
            t
        };
        match n.kind {
            NodeKind::StartEvent(_) => {
                let p = *source.get_or_insert_with(|| {
                    let p = net.add_place(Pos::new(n.pos.x * SCALE - 70.0, n.pos.y * SCALE), 1);
                    net.place_mut(p).unwrap().name = "i".into();
                    p
                });
                let t = new_t(&mut net, &name, n.name.is_empty(), 0.0);
                arc(&mut net, NodeRef::Place(p), NodeRef::Transition(t));
                for q in o {
                    arc(&mut net, NodeRef::Transition(t), NodeRef::Place(q));
                }
            }
            NodeKind::EndEvent(_) => {
                let p = *sink.get_or_insert_with(|| {
                    let p = net.add_place(Pos::new(n.pos.x * SCALE + 70.0, n.pos.y * SCALE), 0);
                    net.place_mut(p).unwrap().name = "o".into();
                    p
                });
                let inputs = if i.is_empty() {
                    vec![None]
                } else {
                    i.into_iter().map(Some).collect()
                };
                let k = inputs.len();
                for (j, q) in inputs.into_iter().enumerate() {
                    let t = new_t(&mut net, &name, n.name.is_empty(), spread(j, k));
                    if let Some(q) = q {
                        arc(&mut net, NodeRef::Place(q), NodeRef::Transition(t));
                    }
                    arc(&mut net, NodeRef::Transition(t), NodeRef::Place(p));
                }
            }
            NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::IntermediateEvent { .. } => {
                if i.len() > 1 {
                    warnings.push(format!("'{name}': several incoming flows, treated as an exclusive merge"));
                }
                let inputs = if i.is_empty() {
                    vec![None]
                } else {
                    i.into_iter().map(Some).collect()
                };
                let k = inputs.len();
                for (j, q) in inputs.into_iter().enumerate() {
                    let t = new_t(&mut net, &name, false, spread(j, k));
                    if let Some(q) = q {
                        arc(&mut net, NodeRef::Place(q), NodeRef::Transition(t));
                    }
                    for &r in &o {
                        arc(&mut net, NodeRef::Transition(t), NodeRef::Place(r));
                    }
                }
            }
            NodeKind::Gateway(GatewayType::Parallel) => {
                let t = new_t(&mut net, &name, true, 0.0);
                for q in i {
                    arc(&mut net, NodeRef::Place(q), NodeRef::Transition(t));
                }
                for r in o {
                    arc(&mut net, NodeRef::Transition(t), NodeRef::Place(r));
                }
            }
            NodeKind::Gateway(g) => {
                if g == GatewayType::Complex {
                    warnings.push(format!("'{name}': complex gateway translated as exclusive"));
                }
                if g == GatewayType::Inclusive && i.len() > 1 {
                    warnings.push(format!("'{name}': inclusive join approximated as an exclusive merge"));
                }
                let out_sets: Vec<Vec<Id>> = if g == GatewayType::Inclusive && o.len() > 1 {
                    if o.len() > 6 {
                        warnings.push(format!("'{name}': too many inclusive branches, translated as exclusive"));
                        o.iter().map(|&r| vec![r]).collect()
                    } else {
                        (1u32..(1 << o.len()))
                            .map(|mask| {
                                o.iter()
                                    .enumerate()
                                    .filter(|(b, _)| mask & (1 << b) != 0)
                                    .map(|(_, &r)| r)
                                    .collect()
                            })
                            .collect()
                    }
                } else if o.is_empty() {
                    vec![vec![]]
                } else {
                    o.iter().map(|&r| vec![r]).collect()
                };
                let in_list: Vec<Option<Id>> = if i.is_empty() {
                    vec![None]
                } else {
                    i.iter().map(|&q| Some(q)).collect()
                };
                let total = in_list.len() * out_sets.len();
                let mut j = 0;
                for q in &in_list {
                    for set in &out_sets {
                        let t = new_t(&mut net, &format!("τ {}", short(&name)), true, spread(j, total));
                        j += 1;
                        if let Some(q) = q {
                            arc(&mut net, NodeRef::Place(*q), NodeRef::Transition(t));
                        }
                        for &r in set {
                            arc(&mut net, NodeRef::Transition(t), NodeRef::Place(r));
                        }
                    }
                }
            }
            NodeKind::DataObject | NodeKind::DataStore | NodeKind::TextAnnotation => {}
        }
    }

    let starts = d.nodes.iter().filter(|n| matches!(n.kind, NodeKind::StartEvent(_))).count();
    let ends = d.nodes.iter().filter(|n| matches!(n.kind, NodeKind::EndEvent(_))).count();
    if starts == 0 || ends == 0 {
        warnings.push(trf("conversion.process_has_start_event_s_end_event", &[&starts, &ends]));
    }
    if d.flows.iter().any(|f| f.kind == FlowKind::Message) {
        warnings.push(tr("conversion.message_flows_ignored_translation").into());
    }
    Conversion { net, warnings }
}

/// Removes the " (Petri net)" suffix added by [`bpmn_to_petri`], in any language.
fn strip_petri_suffix(name: &str) -> String {
    for lang in crate::i18n::Lang::ALL {
        let suffix = crate::i18n::fill(crate::i18n::translate(lang, "conversion.petri_name"), &[&""]);
        if let Some(base) = name.strip_suffix(suffix.as_str()) {
            return base.to_string();
        }
    }
    name.to_string()
}

/// Petri net (workflow net) → BPMN translation, used to "deduce the BPMN diagram"
/// of a discovered net:
///
/// * visible transition → task; silent transition → routing gateway;
/// * several input (output) flows → parallel join (split);
/// * source place → start event, sink place → end event;
///   place names are kept as documentation, not observed business activities;
/// * place with one producer and one consumer → plain sequence flow;
/// * other places → exclusive merge / split;
/// * **decision clusters**: places sharing consumers (non free-choice, e.g. Alpha's
///   `P(A,BE)` and `P(A,CE)`) with identical producers become *one* exclusive
///   decision whose branches are the exact covers of the cluster
///   (`E` alone, or `B` and `C` in parallel); mirrored for merge clusters.
///   This keeps the behaviour of the net instead of creating independent
///   decisions that could deadlock.
pub fn petri_to_bpmn(net: &PetriNet) -> (BpmnDiagram, Vec<String>) {
    use crate::bpmn::{EventTrigger, TaskType};
    const SX: f32 = 1.8;
    const SY: f32 = 1.2;
    let sc = |p: Pos| Pos::new(p.x * SX, p.y * SY);
    let mut d = BpmnDiagram::new(strip_petri_suffix(&net.name));
    let mut warnings = Vec::new();
    if net.arcs.iter().any(|a| a.weight != 1 || a.kind != crate::petri::ArcKind::Normal) {
        warnings.push(tr("conversion.arc_weights_inhibitor_arcs_have_no_bpmn").into());
    }
    if net.places.iter().any(|place| place.capacity.is_some()) {
        warnings.push(tr("conversion.place_capacities_have_no_bpmn").into());
    }

    let np = net.places.len();
    let nt = net.transitions.len();
    let (mut pre_p, mut post_p) = (vec![Vec::new(); np], vec![Vec::new(); np]); // transitions
    let (mut pre_t, mut post_t) = (vec![Vec::new(); nt], vec![Vec::new(); nt]); // places
    for a in &net.arcs {
        match (a.from, a.to) {
            (NodeRef::Transition(t), NodeRef::Place(p)) => {
                let (Some(ti), Some(pi)) = (net.transition_index(t), net.place_index(p)) else {
                    continue;
                };
                pre_p[pi].push(ti);
                post_t[ti].push(pi);
            }
            (NodeRef::Place(p), NodeRef::Transition(t)) => {
                let (Some(ti), Some(pi)) = (net.transition_index(t), net.place_index(p)) else {
                    continue;
                };
                post_p[pi].push(ti);
                pre_t[ti].push(pi);
            }
            _ => {}
        }
    }
    for v in pre_p
        .iter_mut()
        .chain(post_p.iter_mut())
        .chain(pre_t.iter_mut())
        .chain(post_t.iter_mut())
    {
        v.sort();
        v.dedup();
    }

    // ---- clusters
    // split cluster: places connected through shared consumers; join cluster: through shared producers
    let clusters = |link: &Vec<Vec<usize>>| -> Vec<Vec<usize>> {
        let mut parent: Vec<usize> = (0..np).collect();
        fn find(p: &mut Vec<usize>, x: usize) -> usize {
            if p[x] != x {
                let r = find(p, p[x]);
                p[x] = r;
            }
            p[x]
        }
        let mut by_t: HashMap<usize, usize> = HashMap::new();
        for (pl, ts) in link.iter().enumerate() {
            for &t in ts {
                if let Some(&q) = by_t.get(&t) {
                    let (a, b) = (find(&mut parent, pl), find(&mut parent, q));
                    parent[a] = b;
                } else {
                    by_t.insert(t, pl);
                }
            }
        }
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for pl in 0..np {
            let r = find(&mut parent, pl);
            groups.entry(r).or_default().push(pl);
        }
        let mut v: Vec<Vec<usize>> = groups.into_values().filter(|g| g.len() > 1).collect();
        v.sort();
        v
    };
    // Exact covers of the cluster by the (restricted) place sets of the candidate transitions.
    fn covers(universe: &[usize], cands: &[(usize, Vec<usize>)]) -> Vec<Vec<usize>> {
        fn rec(
            universe: &[usize],
            cands: &[(usize, Vec<usize>)],
            covered: &mut Vec<usize>,
            chosen: &mut Vec<usize>,
            out: &mut Vec<Vec<usize>>,
        ) {
            if out.len() > 64 {
                return;
            }
            let Some(&e) = universe.iter().find(|x| !covered.contains(x)) else {
                out.push(chosen.clone());
                return;
            };
            for (t, set) in cands {
                if set.contains(&e) && set.iter().all(|x| !covered.contains(x)) {
                    covered.extend(set);
                    chosen.push(*t);
                    rec(universe, cands, covered, chosen, out);
                    chosen.pop();
                    covered.truncate(covered.len() - set.len());
                }
            }
        }
        let mut out = Vec::new();
        rec(universe, cands, &mut Vec::new(), &mut Vec::new(), &mut out);
        out
    }
    let valid = |k: &Vec<usize>, same: &Vec<Vec<usize>>, other: &Vec<Vec<usize>>, t_side: &Vec<Vec<usize>>| -> Option<Vec<Vec<usize>>> {
        // all places share the same non-empty "same" set (producers for split clusters)
        let s0 = &same[k[0]];
        if s0.is_empty() || k.iter().any(|&p| &same[p] != s0) {
            return None;
        }
        let mut ts: Vec<usize> = k.iter().flat_map(|&p| other[p].clone()).collect();
        ts.sort();
        ts.dedup();
        let cands: Vec<(usize, Vec<usize>)> = ts
            .iter()
            .map(|&t| (t, t_side[t].iter().copied().filter(|p| k.contains(p)).collect()))
            .collect();
        let alts = covers(k, &cands);
        let used: HashSet<usize> = alts.iter().flatten().copied().collect();
        (!alts.is_empty() && ts.iter().all(|t| used.contains(t))).then_some(alts)
    };
    let split_clusters: Vec<(Vec<usize>, Vec<Vec<usize>>)> = clusters(&post_p)
        .into_iter()
        .filter_map(|k| valid(&k, &pre_p, &post_p, &pre_t).map(|a| (k, a)))
        .collect();
    let mut join_clusters: Vec<(Vec<usize>, Vec<Vec<usize>>)> = clusters(&pre_p)
        .into_iter()
        .filter_map(|k| valid(&k, &post_p, &pre_p, &post_t).map(|a| (k, a)))
        .collect();
    let in_split: HashSet<usize> = split_clusters.iter().flat_map(|(k, _)| k.clone()).collect();
    join_clusters.retain(|(k, _)| k.iter().all(|p| !in_split.contains(p)));
    let in_join: HashSet<usize> = join_clusters.iter().flat_map(|(k, _)| k.clone()).collect();

    // ---- nodes & edges
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Ep {
        Node(Id),
        In(usize),
        Out(usize),
    }
    let mut edges: Vec<(Ep, Ep)> = Vec::new();
    let tpos: Vec<Pos> = net.transitions.iter().map(|t| sc(t.pos)).collect();
    let ppos: Vec<Pos> = net.places.iter().map(|p| sc(p.pos)).collect();
    let avg = |pts: &[Pos]| {
        let k = pts.len().max(1) as f32;
        Pos::new(pts.iter().map(|p| p.x).sum::<f32>() / k, pts.iter().map(|p| p.y).sum::<f32>() / k)
    };
    let gw = |d: &mut BpmnDiagram, g: GatewayType, pos: Pos| d.add_node(NodeKind::Gateway(g), pos);

    for (k, alts) in &split_clusters {
        let c = avg(&k.iter().map(|&p| ppos[p]).collect::<Vec<_>>());
        let x = gw(&mut d, GatewayType::Exclusive, c);
        let producers = &pre_p[k[0]];
        let entry = if producers.len() > 1 {
            let j = gw(&mut d, GatewayType::Exclusive, Pos::new(c.x - 70.0, c.y));
            edges.push((Ep::Node(j), Ep::Node(x)));
            j
        } else {
            x
        };
        for &t in producers {
            edges.push((Ep::Out(t), Ep::Node(entry)));
        }
        for alt in alts {
            if alt.len() == 1 {
                edges.push((Ep::Node(x), Ep::In(alt[0])));
            } else {
                let at = avg(&alt.iter().map(|&t| tpos[t]).collect::<Vec<_>>());
                let g = gw(&mut d, GatewayType::Parallel, Pos::new((c.x + at.x) / 2.0, at.y));
                edges.push((Ep::Node(x), Ep::Node(g)));
                for &t in alt {
                    edges.push((Ep::Node(g), Ep::In(t)));
                }
            }
        }
    }
    for (k, alts) in &join_clusters {
        let c = avg(&k.iter().map(|&p| ppos[p]).collect::<Vec<_>>());
        let j = gw(&mut d, GatewayType::Exclusive, c);
        let consumers = &post_p[k[0]];
        let exit = if consumers.len() > 1 {
            let x = gw(&mut d, GatewayType::Exclusive, Pos::new(c.x + 70.0, c.y));
            edges.push((Ep::Node(j), Ep::Node(x)));
            x
        } else {
            j
        };
        for &t in consumers {
            edges.push((Ep::Node(exit), Ep::In(t)));
        }
        for alt in alts {
            if alt.len() == 1 {
                edges.push((Ep::Out(alt[0]), Ep::Node(j)));
            } else {
                let at = avg(&alt.iter().map(|&t| tpos[t]).collect::<Vec<_>>());
                let g = gw(&mut d, GatewayType::Parallel, Pos::new((c.x + at.x) / 2.0, at.y));
                edges.push((Ep::Node(g), Ep::Node(j)));
                for &t in alt {
                    edges.push((Ep::Out(t), Ep::Node(g)));
                }
            }
        }
    }
    for p in 0..np {
        if in_split.contains(&p) || in_join.contains(&p) {
            continue;
        }
        let (pre, post) = (&pre_p[p], &post_p[p]);
        let pos = ppos[p];
        let name = net.places[p].name.clone();
        match (pre.len(), post.len()) {
            (0, 0) => {}
            (1, 1) => edges.push((Ep::Out(pre[0]), Ep::In(post[0]))),
            (0, _) => {
                let s = d.add_node(NodeKind::StartEvent(EventTrigger::None), pos);
                d.node_mut(s).unwrap().documentation = name;
                let exit = if post.len() > 1 {
                    let x = gw(&mut d, GatewayType::Exclusive, Pos::new(pos.x + 70.0, pos.y));
                    edges.push((Ep::Node(s), Ep::Node(x)));
                    x
                } else {
                    s
                };
                for &t in post {
                    edges.push((Ep::Node(exit), Ep::In(t)));
                }
            }
            (_, 0) => {
                let e = d.add_node(NodeKind::EndEvent(EventTrigger::None), pos);
                d.node_mut(e).unwrap().documentation = name;
                let entry = if pre.len() > 1 {
                    let j = gw(&mut d, GatewayType::Exclusive, Pos::new(pos.x - 70.0, pos.y));
                    edges.push((Ep::Node(j), Ep::Node(e)));
                    j
                } else {
                    e
                };
                for &t in pre {
                    edges.push((Ep::Out(t), Ep::Node(entry)));
                }
            }
            _ => {
                let both = pre.len() > 1 && post.len() > 1;
                let j = (pre.len() > 1).then(|| {
                    gw(
                        &mut d,
                        GatewayType::Exclusive,
                        Pos::new(pos.x - if both { 40.0 } else { 0.0 }, pos.y),
                    )
                });
                let x = (post.len() > 1).then(|| {
                    gw(
                        &mut d,
                        GatewayType::Exclusive,
                        Pos::new(pos.x + if both { 40.0 } else { 0.0 }, pos.y),
                    )
                });
                if let (Some(j), Some(x)) = (j, x) {
                    edges.push((Ep::Node(j), Ep::Node(x)));
                }
                let (entry, exit) = match (j, x) {
                    (Some(j), Some(x)) => (j, x),
                    (Some(j), None) => (j, j),
                    (None, Some(x)) => (x, x),
                    (None, None) => unreachable!(),
                };
                for &t in pre {
                    edges.push((Ep::Out(t), Ep::Node(entry)));
                }
                for &t in post {
                    edges.push((Ep::Node(exit), Ep::In(t)));
                }
            }
        }
    }

    // ---- activities and silent routing (+ explicit parallel joins / splits)
    let mut t_in = vec![0; nt];
    let mut t_out = vec![0; nt];
    let mut silent_connectors = Vec::new();
    for (t, tr) in net.transitions.iter().enumerate() {
        let kind = if tr.silent {
            NodeKind::Gateway(GatewayType::Parallel)
        } else {
            NodeKind::Task(TaskType::Abstract)
        };
        let task = d.add_node(kind, tpos[t]);
        if tr.silent {
            d.node_mut(task).unwrap().documentation = tr.name.clone();
            silent_connectors.push(task);
        } else {
            d.node_mut(task).unwrap().name = tr.name.clone();
        }
        let ins = edges.iter().filter(|e| e.1 == Ep::In(t)).count();
        let outs = edges.iter().filter(|e| e.0 == Ep::Out(t)).count();
        t_in[t] = task;
        t_out[t] = task;
        if ins > 1 {
            let g = gw(&mut d, GatewayType::Parallel, Pos::new(tpos[t].x - 105.0, tpos[t].y));
            let _ = d.add_flow(g, task);
            t_in[t] = g;
        }
        if outs > 1 {
            let g = gw(&mut d, GatewayType::Parallel, Pos::new(tpos[t].x + 105.0, tpos[t].y));
            let _ = d.add_flow(task, g);
            t_out[t] = g;
        }
    }
    let resolve = |e: Ep| match e {
        Ep::Node(id) => id,
        Ep::In(t) => t_in[t],
        Ep::Out(t) => t_out[t],
    };
    for (a, b) in edges {
        let _ = d.add_flow(resolve(a), resolve(b));
    }
    contract_silent_connectors(&mut d, &silent_connectors);
    if (0..np).filter(|&p| pre_p[p].is_empty()).count() != 1 {
        warnings.push(tr("conversion.net_has_no_single_source_place_start").into());
    }
    (d, warnings)
}

/// Remove only transparent routing introduced for a silent transition. Keep a
/// gateway when the replacement would become a forbidden self-loop or merge
/// distinct parallel routes into an existing flow. Choice clusters and the
/// explicit AND joins/splits are deliberately outside this reduction.
fn contract_silent_connectors(diagram: &mut BpmnDiagram, connectors: &[Id]) {
    for &id in connectors {
        let incoming: Vec<_> = diagram.incoming(id).cloned().collect();
        let outgoing: Vec<_> = diagram.outgoing(id).cloned().collect();
        let ([before], [after]) = (incoming.as_slice(), outgoing.as_slice()) else {
            continue;
        };
        if before.from == after.to
            || !before.name.is_empty()
            || !after.name.is_empty()
            || before.is_default
            || after.is_default
            || diagram.flows.iter().any(|flow| {
                (flow.from == before.from && flow.to == after.to) || ((flow.from == id || flow.to == id) && flow.kind != FlowKind::Sequence)
            })
        {
            continue;
        }
        if diagram.add_flow(before.from, after.to).is_err() {
            continue;
        }
        let documentation = diagram.node(id).map(|node| node.documentation.clone()).unwrap_or_default();
        diagram.remove_node(id);
        if !documentation.is_empty()
            && let Some(target) = diagram.node_mut(after.to)
        {
            if !target.documentation.is_empty() {
                target.documentation.push('\n');
            }
            target.documentation.push_str(&documentation);
        }
    }
}

fn spread(j: usize, k: usize) -> f32 {
    (j as f32 - (k as f32 - 1.0) / 2.0) * 50.0
}

fn short(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() > 18 {
        format!("{}…", s.chars().take(17).collect::<String>())
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Soundness, soundness};
    use crate::bpmn::{EventTrigger, TaskType};
    use std::collections::{BTreeSet, VecDeque};

    /// Compare complete observable words, including rejected interleavings, on
    /// bounded workflow examples. Silent cycles terminate through state reuse.
    fn bounded_language(net: &PetriNet, max_word: usize) -> BTreeSet<Vec<String>> {
        let compiled = net.compile();
        let sinks: Vec<_> = net
            .places
            .iter()
            .enumerate()
            .filter(|(_, place)| !net.arcs.iter().any(|arc| arc.from == NodeRef::Place(place.id)))
            .map(|(index, _)| index)
            .collect();
        let initial = (net.initial_marking(), Vec::<String>::new());
        let mut seen = HashSet::from([initial.clone()]);
        let mut queue = VecDeque::from([initial]);
        let mut language = BTreeSet::new();
        while let Some((marking, word)) = queue.pop_front() {
            if marking
                .iter()
                .enumerate()
                .all(|(index, &tokens)| tokens == 0 || sinks.contains(&index))
                && sinks.iter().map(|&index| marking[index]).sum::<u32>() == 1
            {
                language.insert(word.clone());
            }
            for (index, transition) in net.transitions.iter().enumerate() {
                if !transition.silent && word.len() == max_word {
                    continue;
                }
                if let Some(marking) = compiled.fire(index, &marking) {
                    let mut next_word = word.clone();
                    if !transition.silent {
                        next_word.push(transition.name.clone());
                    }
                    let state = (marking, next_word);
                    if seen.insert(state.clone()) {
                        assert!(seen.len() <= 20_000, "conversion example exceeded its state bound");
                        queue.push_back(state);
                    }
                }
            }
        }
        language
    }

    fn assert_round_trip_language(net: &PetriNet, max_word: usize) -> BpmnDiagram {
        let original = net.clone();
        let expected = bounded_language(net, max_word);
        assert!(!expected.is_empty(), "the original example must have a complete trace");
        let (diagram, warnings) = petri_to_bpmn(net);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(net, &original, "conversion must not alter the source document");
        assert!(
            diagram
                .nodes
                .iter()
                .filter(|node| matches!(node.kind, NodeKind::Task(_)))
                .all(|node| !node.name.is_empty())
        );
        assert!(
            diagram
                .validate()
                .iter()
                .all(|issue| issue.severity != crate::bpmn::Severity::Error),
            "{:?}",
            diagram.validate()
        );
        let converted = bpmn_to_petri(&diagram);
        assert!(converted.warnings.is_empty(), "{:?}", converted.warnings);
        assert_eq!(bounded_language(&converted.net, max_word), expected);
        diagram
    }

    fn transition(net: &mut PetriNet, name: &str, silent: bool) -> NodeRef {
        let id = net.add_transition(Pos::new(net.transitions.len() as f32 * 100.0, 100.0));
        let transition = net.transition_mut(id).unwrap();
        transition.name = name.into();
        transition.silent = silent;
        NodeRef::Transition(id)
    }

    fn place(net: &mut PetriNet, tokens: u32) -> NodeRef {
        NodeRef::Place(net.add_place(Pos::new(net.places.len() as f32 * 100.0, 0.0), tokens))
    }

    #[test]
    fn projected_routing_and_boundaries_do_not_become_business_tasks() {
        use crate::mining::{DiscoveryOptions, EventLog, discover_with_options_cancellable};
        let log = EventLog {
            traces: vec![
                vec!["Register".into(), "Vide1".into(), "Empty the oven".into(), "Pay".into()],
                vec!["Register".into(), "Empty2".into(), "Pay".into()],
            ],
        };
        let discovered = discover_with_options_cancellable(
            &log,
            &DiscoveryOptions {
                normalize_boundaries: true,
            },
            &|| false,
        )
        .unwrap();
        let diagram = assert_round_trip_language(&discovered.net, 3);
        let tasks: BTreeSet<_> = diagram
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::Task(_)))
            .map(|node| node.name.as_str())
            .collect();
        assert_eq!(tasks, BTreeSet::from(["Register", "Empty the oven", "Pay"]));
        assert!(
            diagram
                .nodes
                .iter()
                .filter(|node| matches!(node.kind, NodeKind::StartEvent(_) | NodeKind::EndEvent(_)))
                .all(|node| node.name.is_empty() && !node.documentation.is_empty())
        );
        let back = bpmn_to_petri(&diagram).net;
        for trace in &log.traces {
            crate::mining::replay(&back, &discovered.project_trace(trace)).unwrap();
        }
    }

    #[test]
    fn normalized_parallel_final_activities_still_require_both_branches() {
        use crate::mining::{DiscoveryOptions, EventLog, discover_with_options_cancellable};
        let log = EventLog::parse("AB BA").unwrap();
        let discovered = discover_with_options_cancellable(
            &log,
            &DiscoveryOptions {
                normalize_boundaries: true,
            },
            &|| false,
        )
        .unwrap();
        let diagram = assert_round_trip_language(&discovered.net, 3);
        let back = bpmn_to_petri(&diagram).net;
        assert_eq!(
            bounded_language(&back, 3),
            BTreeSet::from([vec!["A".to_owned(), "B".to_owned()], vec!["B".to_owned(), "A".to_owned()]])
        );
        assert!(matches!(soundness(&back, 10_000), Soundness::Sound));
    }

    #[test]
    fn silent_retry_routing_preserves_repeated_business_activity() {
        let mut net = PetriNet::new("Retry");
        let input = place(&mut net, 1);
        let ready = place(&mut net, 0);
        let cooked = place(&mut net, 0);
        let output = place(&mut net, 0);
        let enter = transition(&mut net, "Enter", true);
        let cook = transition(&mut net, "Cook", false);
        let retry = transition(&mut net, "Retry route", true);
        let serve = transition(&mut net, "Serve", false);
        for (from, to) in [
            (input, enter),
            (enter, ready),
            (ready, cook),
            (cook, cooked),
            (cooked, retry),
            (retry, ready),
            (cooked, serve),
            (serve, output),
        ] {
            net.add_arc(from, to).unwrap();
        }
        let diagram = assert_round_trip_language(&net, 4);
        let language = bounded_language(&bpmn_to_petri(&diagram).net, 4);
        assert!(language.contains(&vec!["Cook".into(), "Cook".into(), "Serve".into()]));
        assert!(!language.contains(&vec!["Serve".into()]));
    }

    #[test]
    fn empty_parallel_routes_are_not_collapsed_into_one_flow() {
        let mut net = PetriNet::new("Parallel routing");
        let input = place(&mut net, 1);
        let left = place(&mut net, 0);
        let right = place(&mut net, 0);
        let left_done = place(&mut net, 0);
        let right_done = place(&mut net, 0);
        let ready = place(&mut net, 0);
        let output = place(&mut net, 0);
        let fork = transition(&mut net, "Fork", true);
        let left_route = transition(&mut net, "Left route", true);
        let right_route = transition(&mut net, "Right route", true);
        let join = transition(&mut net, "Join", true);
        let activity = transition(&mut net, "Finish", false);
        for (from, to) in [
            (input, fork),
            (fork, left),
            (fork, right),
            (left, left_route),
            (left_route, left_done),
            (right, right_route),
            (right_route, right_done),
            (left_done, join),
            (right_done, join),
            (join, ready),
            (ready, activity),
            (activity, output),
        ] {
            net.add_arc(from, to).unwrap();
        }
        let diagram = assert_round_trip_language(&net, 2);
        assert!(
            diagram
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Gateway(GatewayType::Parallel) && diagram.outgoing(node.id).count() == 2)
        );
        assert!(
            diagram
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Gateway(GatewayType::Parallel) && diagram.incoming(node.id).count() == 2)
        );
    }

    #[test]
    fn capacities_are_reported_without_modifying_the_original_net() {
        let mut net = crate::mining::alpha(&crate::mining::EventLog::parse("AB").unwrap()).net;
        net.places[0].capacity = Some(1);
        let original = net.clone();
        let (_, warnings) = petri_to_bpmn(&net);
        assert!(
            warnings
                .iter()
                .any(|warning| warning == tr("conversion.place_capacities_have_no_bpmn"))
        );
        assert_eq!(net, original);
    }

    #[test]
    fn xor_and_process_is_sound() {
        let mut d = BpmnDiagram::new("p");
        let s = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::new(0.0, 0.0));
        let split = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::new(100.0, 0.0));
        let a = d.add_node(NodeKind::Task(TaskType::User), Pos::new(200.0, -80.0));
        let x = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(200.0, 80.0));
        let b = d.add_node(NodeKind::Task(TaskType::User), Pos::new(300.0, 40.0));
        let c = d.add_node(NodeKind::Task(TaskType::User), Pos::new(300.0, 120.0));
        let xj = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(400.0, 80.0));
        let join = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::new(500.0, 0.0));
        let e = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::new(600.0, 0.0));
        for (p, q) in [
            (s, split),
            (split, a),
            (split, x),
            (x, b),
            (x, c),
            (b, xj),
            (c, xj),
            (a, join),
            (xj, join),
            (join, e),
        ] {
            d.add_flow(p, q).unwrap();
        }
        let conv = bpmn_to_petri(&d);
        assert!(conv.warnings.is_empty(), "{:?}", conv.warnings);
        assert!(matches!(soundness(&conv.net, 10_000), Soundness::Sound));
    }

    #[test]
    fn deadlocking_process_is_unsound() {
        // XOR split joined by an AND join → deadlock
        let mut d = BpmnDiagram::new("p");
        let s = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::new(0.0, 0.0));
        let x = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(100.0, 0.0));
        let a = d.add_node(NodeKind::Task(TaskType::User), Pos::new(200.0, -80.0));
        let b = d.add_node(NodeKind::Task(TaskType::User), Pos::new(200.0, 80.0));
        let j = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::new(300.0, 0.0));
        let e = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::new(400.0, 0.0));
        for (p, q) in [(s, x), (x, a), (x, b), (a, j), (b, j), (j, e)] {
            d.add_flow(p, q).unwrap();
        }
        let conv = bpmn_to_petri(&d);
        assert!(matches!(soundness(&conv.net, 10_000), Soundness::Unsound(_)));
    }

    #[test]
    fn alpha_net_to_bpmn() {
        use crate::mining::{EventLog, alpha};
        // Course example 1: A splits into B and C in parallel; E F is an alternative path.
        let log = EventLog::parse("ABCD ACBD EF").unwrap();
        let r = alpha(&log);
        let (d, _) = petri_to_bpmn(&r.net);
        let tasks = d.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Task(_))).count();
        let and = d
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Gateway(GatewayType::Parallel))
            .count();
        let xor = d
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Gateway(GatewayType::Exclusive))
            .count();
        assert_eq!(tasks, 6);
        assert_eq!(and, 2, "AND split after A, AND join before D");
        assert_eq!(xor, 2, "XOR split after start (A or E), XOR merge before end (D or F)");
        assert!(
            d.validate().iter().all(|i| i.severity != crate::bpmn::Severity::Error),
            "{:?}",
            d.validate()
        );
        // Round trip: the BPMN translates back to a sound workflow net
        let back = bpmn_to_petri(&d);
        assert!(matches!(
            crate::analysis::soundness(&back.net, 10_000),
            crate::analysis::Soundness::Sound
        ));
    }

    /// Course example 2 (ABCD, ACBD, AED): non free-choice Alpha net.
    #[test]
    fn non_free_choice_alpha_net_to_bpmn() {
        use crate::mining::{EventLog, alpha};
        let r = alpha(&EventLog::parse("ABCD ACBD AED").unwrap());
        let (d, _) = petri_to_bpmn(&r.net);
        let count = |k: NodeKind| d.nodes.iter().filter(|n| n.kind == k).count();
        // A → XOR [ E | AND(B, C) ] → XOR → D
        assert_eq!(count(NodeKind::Gateway(GatewayType::Exclusive)), 2);
        assert_eq!(count(NodeKind::Gateway(GatewayType::Parallel)), 2);
        assert!(
            d.validate().iter().all(|i| i.severity != crate::bpmn::Severity::Error),
            "{:?}",
            d.validate()
        );
        let back = bpmn_to_petri(&d);
        assert!(matches!(
            crate::analysis::soundness(&back.net, 10_000),
            crate::analysis::Soundness::Sound
        ));
        assert_round_trip_language(&r.net, 4);
    }
}
