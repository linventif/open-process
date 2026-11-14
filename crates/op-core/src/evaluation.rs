//! Exact cost / elapsed-time evaluation of acyclic, structured BPMN processes.
//!
//! Choices are equiprobable locally. Parallel interleavings are traces, not
//! additional random choices. We expand choices before taking parallel maxima:
//! E[max(X,Y)] is generally different from max(E[X],E[Y]).

use crate::bpmn::{BpmnDiagram, FlowKind, GatewayType, Id, NodeKind};
use crate::i18n::tr;
use std::collections::{BTreeSet, HashMap, VecDeque};

const MAX_NODES: usize = 256;
const MAX_SCENARIOS: usize = 256;
const MAX_TRACES: usize = 512;

#[derive(Clone, Debug)]
enum Expr {
    Activity(Id),
    Sequence(Vec<Expr>),
    Parallel(Vec<Expr>),
    Choice(Vec<Expr>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    /// Probability = 1 / denominator; every encountered XOR contributes its arity.
    pub denominator: usize,
    pub cost_formula: String,
    pub duration_formula: String,
    pub cost: f64,
    pub duration: f64,
    pub traces: Vec<Vec<Id>>,
    pub traces_truncated: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Evaluation {
    pub scenarios: Vec<Scenario>,
    pub mean_duration: f64,
    pub duration_formula: String,
}

/// Stable exercise symbols, independent of duplicate, empty or translated names.
pub fn activities(d: &BpmnDiagram) -> impl Iterator<Item = (Id, String)> + '_ {
    d.nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess))
        .enumerate()
        .map(|(i, n)| (n.id, format!("A{}", i + 1)))
}

/// Build a structured expression using immediate postdominators as joins.
fn expression(d: &BpmnDiagram) -> Result<Expr, String> {
    let nodes: Vec<_> = d.nodes.iter().filter(|n| n.kind.is_flow_node()).collect();
    let n = nodes.len();
    let unsupported = || tr("evaluation.evaluation_requires_acyclic_process_structured_xor_gateways").to_string();
    if n == 0 || n > MAX_NODES {
        return Err(tr("evaluation.evaluation_supports_1_256_flow_nodes").into());
    }
    let index: HashMap<_, _> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
    if index.len() != n || d.flows.iter().any(|f| f.kind == FlowKind::Message) {
        return Err(unsupported());
    }
    let mut out = vec![Vec::new(); n + 1];
    let mut incoming = vec![0; n + 1];
    for f in d.flows.iter().filter(|f| f.kind == FlowKind::Sequence) {
        let (Some(&a), Some(&b)) = (index.get(&f.from), index.get(&f.to)) else {
            return Err(unsupported());
        };
        out[a].push(b);
        incoming[b] += 1;
    }
    let starts: Vec<_> = (0..n).filter(|&i| matches!(nodes[i].kind, NodeKind::StartEvent(_))).collect();
    if starts.len() != 1 || incoming[starts[0]] != 0 {
        return Err(unsupported());
    }
    for i in 0..n {
        if i != starts[0] && incoming[i] == 0 {
            return Err(unsupported());
        }
        if matches!(nodes[i].kind, NodeKind::EndEvent(_)) {
            if !out[i].is_empty() {
                return Err(unsupported());
            }
            // A virtual sink also permits exclusive alternatives with separate ends.
            out[i].push(n);
            incoming[n] += 1;
        } else if out[i].is_empty() {
            return Err(unsupported());
        }
        if matches!(nodes[i].kind, NodeKind::Gateway(g) if !matches!(g, GatewayType::Exclusive | GatewayType::Parallel)) {
            return Err(tr("evaluation.evaluation_supports_xor_gateways_event_based_complex").into());
        }
        if matches!(nodes[i].kind, NodeKind::EndEvent(t) if t != crate::bpmn::EventTrigger::None)
            || matches!(nodes[i].kind, NodeKind::IntermediateEvent { trigger, .. } if trigger != crate::bpmn::EventTrigger::None)
        {
            return Err(tr("evaluation.evaluation_assumes_zero_duration_events_without_timers").into());
        }
    }
    let mut degree = incoming.clone();
    let mut queue = VecDeque::from([starts[0]]);
    let mut order = Vec::new();
    while let Some(i) = queue.pop_front() {
        order.push(i);
        for &j in &out[i] {
            degree[j] -= 1;
            if degree[j] == 0 {
                queue.push_back(j);
            }
        }
    }
    if order.len() != n + 1 {
        return Err(unsupported());
    }
    let mut post = vec![BTreeSet::new(); n + 1];
    post[n].insert(n);
    for &i in order.iter().rev().skip(1) {
        let mut common = post[out[i][0]].clone();
        for &j in &out[i][1..] {
            common.retain(|x| post[j].contains(x));
        }
        common.insert(i);
        post[i] = common;
    }

    struct Builder<'a> {
        nodes: Vec<&'a crate::bpmn::Node>,
        out: Vec<Vec<usize>>,
        incoming: Vec<usize>,
        post: Vec<BTreeSet<usize>>,
        order: Vec<usize>,
        seen: BTreeSet<usize>,
        joins: BTreeSet<usize>,
    }
    impl Builder<'_> {
        fn region(&mut self, mut i: usize, stop: usize) -> Result<Expr, String> {
            let mut seq = Vec::new();
            while i != stop {
                let error = || tr("evaluation.branches_must_have_matching_properly_nested_xor").to_string();
                if i == self.nodes.len() || !self.seen.insert(i) {
                    return Err(error());
                }
                let node = self.nodes[i];
                if self.incoming[i] > 1 && !self.joins.contains(&i) {
                    return Err(error());
                }
                if matches!(node.kind, NodeKind::Task(_) | NodeKind::SubProcess) {
                    seq.push(Expr::Activity(node.id));
                }
                let targets = self.out[i].clone();
                if targets.len() == 1 {
                    i = targets[0];
                    continue;
                }
                let NodeKind::Gateway(g) = node.kind else { return Err(error()) };
                let join = *self
                    .order
                    .iter()
                    .find(|&&j| j != i && self.post[i].contains(&j))
                    .ok_or_else(error)?;
                if join == self.nodes.len() {
                    if g != GatewayType::Exclusive {
                        return Err(error());
                    }
                } else if self.nodes[join].kind != NodeKind::Gateway(g) || !self.joins.insert(join) {
                    return Err(error());
                }
                let branches = targets.into_iter().map(|j| self.region(j, join)).collect::<Result<Vec<_>, _>>()?;
                seq.push(if g == GatewayType::Parallel {
                    Expr::Parallel(branches)
                } else {
                    Expr::Choice(branches)
                });
                i = join;
            }
            Ok(Expr::Sequence(seq))
        }
    }
    let mut builder = Builder {
        nodes,
        out,
        incoming,
        post,
        order,
        seen: BTreeSet::new(),
        joins: BTreeSet::new(),
    };
    let result = builder.region(starts[0], n)?;
    if builder.seen.len() != n {
        return Err(unsupported());
    }
    Ok(result)
}

/// Expand only XOR decisions. Parallel scheduling contributes no probabilities.
fn outcomes(e: &Expr) -> Result<Vec<(usize, Expr)>, String> {
    let limit = || tr("evaluation.evaluation_exceeds_limit_256_choice_scenarios_simplify").to_string();
    match e {
        Expr::Activity(_) => Ok(vec![(1, e.clone())]),
        Expr::Choice(v) => {
            let mut result = Vec::new();
            for branch in v {
                for (denom, plan) in outcomes(branch)? {
                    result.push((denom.checked_mul(v.len()).ok_or_else(limit)?, plan));
                    if result.len() > MAX_SCENARIOS {
                        return Err(limit());
                    }
                }
            }
            Ok(result)
        }
        Expr::Sequence(v) | Expr::Parallel(v) => {
            let mut result = vec![(1usize, Vec::new())];
            for part in v {
                let next = outcomes(part)?;
                if result.len() * next.len() > MAX_SCENARIOS {
                    return Err(limit());
                }
                let mut product = Vec::new();
                for (a, prefix) in result {
                    for (b, suffix) in &next {
                        let mut parts = prefix.clone();
                        parts.push(suffix.clone());
                        product.push((a.checked_mul(*b).ok_or_else(limit)?, parts));
                    }
                }
                result = product;
            }
            Ok(result
                .into_iter()
                .map(|(p, v)| {
                    (
                        p,
                        if matches!(e, Expr::Parallel(_)) {
                            Expr::Parallel(v)
                        } else {
                            Expr::Sequence(v)
                        },
                    )
                })
                .collect())
        }
    }
}

fn terms(e: &Expr, d: &BpmnDiagram, symbols: &HashMap<Id, String>, duration: bool) -> (String, f64) {
    match e {
        Expr::Activity(id) => {
            let n = d.node(*id).unwrap();
            (
                format!("{}{}", if duration { "T" } else { "C" }, symbols[id]),
                if duration { n.duration } else { n.cost },
            )
        }
        Expr::Sequence(v) | Expr::Parallel(v) => {
            let parts: Vec<_> = v.iter().map(|e| terms(e, d, symbols, duration)).filter(|(s, _)| s != "0").collect();
            if parts.is_empty() {
                return ("0".into(), 0.0);
            }
            if parts.len() == 1 {
                return parts.into_iter().next().unwrap();
            }
            if duration && matches!(e, Expr::Parallel(_)) {
                (
                    format!("max({})", parts.iter().map(|p| p.0.as_str()).collect::<Vec<_>>().join(", ")),
                    parts.iter().map(|p| p.1).fold(0.0, f64::max),
                )
            } else {
                (
                    parts.iter().map(|p| p.0.as_str()).collect::<Vec<_>>().join(" + "),
                    parts.iter().map(|p| p.1).sum(),
                )
            }
        }
        Expr::Choice(_) => unreachable!("choices have been expanded"),
    }
}

/// Generate the serial words of a partial order, stopping after limit + 1.
fn words(e: &Expr, limit: usize) -> (Vec<Vec<Id>>, bool) {
    fn shuffle(a: &[Id], b: &[Id], prefix: &mut Vec<Id>, out: &mut Vec<Vec<Id>>, cap: usize) {
        if out.len() >= cap {
            return;
        }
        if a.is_empty() && b.is_empty() {
            out.push(prefix.clone());
            return;
        }
        if let Some((&head, tail)) = a.split_first() {
            prefix.push(head);
            shuffle(tail, b, prefix, out, cap);
            prefix.pop();
        }
        if let Some((&head, tail)) = b.split_first() {
            prefix.push(head);
            shuffle(a, tail, prefix, out, cap);
            prefix.pop();
        }
    }
    match e {
        Expr::Activity(id) => (vec![vec![*id]], false),
        Expr::Sequence(v) | Expr::Parallel(v) => {
            let mut result = vec![Vec::new()];
            let mut truncated = false;
            for part in v {
                let (next, cut) = words(part, limit);
                truncated |= cut;
                let mut product = Vec::new();
                'outer: for a in &result {
                    for b in &next {
                        if matches!(e, Expr::Parallel(_)) {
                            shuffle(a, b, &mut Vec::new(), &mut product, limit + 1);
                        } else {
                            product.push(a.iter().chain(b).copied().collect());
                        }
                        if product.len() > limit {
                            break 'outer;
                        }
                    }
                }
                truncated |= product.len() > limit;
                product.truncate(limit);
                result = product;
            }
            (result, truncated)
        }
        Expr::Choice(_) => unreachable!("choices have been expanded"),
    }
}

pub fn evaluate(d: &BpmnDiagram) -> Result<Evaluation, String> {
    for n in d
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess))
    {
        if [n.cost, n.duration].iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(tr("evaluation.activity_costs_durations_must_finite_non_negative").into());
        }
    }
    let plans = outcomes(&expression(d)?)?;
    let symbols = activities(d).collect();
    let per_scenario = MAX_TRACES / plans.len();
    let mut scenarios = Vec::new();
    for (denominator, plan) in plans {
        let (cost_formula, cost) = terms(&plan, d, &symbols, false);
        let (duration_formula, duration) = terms(&plan, d, &symbols, true);
        if !cost.is_finite() || !duration.is_finite() {
            return Err(tr("evaluation.calculated_cost_duration_exceeds_numeric_range").into());
        }
        let (traces, traces_truncated) = words(&plan, per_scenario);
        scenarios.push(Scenario {
            denominator,
            cost_formula,
            duration_formula,
            cost,
            duration,
            traces,
            traces_truncated,
        });
    }
    let mean_duration = scenarios.iter().map(|s| s.duration / s.denominator as f64).sum();
    let duration_formula = format!(
        "T(P) = {}",
        scenarios
            .iter()
            .map(|s| {
                if s.denominator == 1 {
                    s.duration_formula.clone()
                } else {
                    format!("({}) / {}", s.duration_formula, s.denominator)
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
    );
    Ok(Evaluation {
        scenarios,
        mean_duration,
        duration_formula,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bpmn::*,
        document::{self, Model},
        examples,
        geom::Pos,
    };

    #[test]
    fn course_process_traces_cost_and_expected_maximum() {
        let mut d = examples::evaluation_process();
        for (i, node) in d.nodes.iter_mut().take(8).enumerate() {
            node.cost = (i + 1) as f64;
            node.duration = 1.0;
        }
        d.nodes[4].duration = 10.0;
        d.nodes[5].duration = 2.0;
        d.nodes[6].duration = 5.0;
        let r = evaluate(&d).unwrap();
        assert_eq!(r.scenarios.len(), 2);
        assert_eq!(r.scenarios.iter().map(|s| s.traces.len()).sum::<usize>(), 60);
        assert_eq!(r.scenarios[0].cost, 30.0);
        assert_eq!(r.scenarios[1].cost, 31.0);
        assert_eq!(r.scenarios[0].duration, 13.0);
        assert_eq!(r.scenarios[1].duration, 7.0);
        assert_eq!(r.mean_duration, 10.0, "average the maxima, not the branch durations");
        assert_eq!(r.scenarios[0].duration_formula, "TA1 + max(TA2 + TA5, TA3 + TA4, TA7) + TA8");
        assert!(r.duration_formula.contains("/ 2"));
        let mut all = BTreeSet::new();
        for (i, s) in r.scenarios.iter().enumerate() {
            assert_eq!(s.denominator, 2);
            assert!(!s.traces_truncated);
            for trace in &s.traces {
                assert!(all.insert(trace.clone()), "no repeated interleavings");
                assert_eq!(trace.len(), 7);
                assert_eq!(trace[0], d.nodes[0].id);
                assert_eq!(trace[6], d.nodes[7].id);
                let at = |j: usize| trace.iter().position(|id| *id == d.nodes[j].id).unwrap();
                assert!(at(1) < at(4 + i));
                assert!(at(2) < at(3));
            }
        }
    }

    #[test]
    fn probabilities_are_local_choices_not_numbers_of_traces() {
        let mut d = examples::order_process();
        for n in &mut d.nodes {
            if matches!(n.kind, NodeKind::Task(_)) {
                n.duration = 1.0;
            }
        }
        let r = evaluate(&d).unwrap();
        assert_eq!(r.scenarios.iter().map(|s| s.traces.len()).collect::<Vec<_>>(), vec![1, 3]);
        assert!(r.scenarios.iter().all(|s| s.denominator == 2));
        assert_eq!(r.mean_duration, 2.5);
        let e = Expr::Choice(vec![Expr::Activity(1), Expr::Choice(vec![Expr::Activity(2), Expr::Activity(3)])]);
        assert_eq!(outcomes(&e).unwrap().iter().map(|s| s.0).collect::<Vec<_>>(), vec![2, 4, 4]);
    }

    #[test]
    fn trace_limit_keeps_cost_and_time_exact() {
        let mut d = BpmnDiagram::new("wide AND");
        let s = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::default());
        let fork = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::default());
        let join = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::default());
        let end = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::default());
        d.add_flow(s, fork).unwrap();
        d.add_flow(join, end).unwrap();
        for _ in 0..8 {
            let a = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::default());
            let node = d.node_mut(a).unwrap();
            node.cost = 2.0;
            node.duration = 3.0;
            d.add_flow(fork, a).unwrap();
            d.add_flow(a, join).unwrap();
        }
        let r = evaluate(&d).unwrap();
        assert_eq!(r.mean_duration, 3.0);
        assert_eq!(r.scenarios[0].cost, 16.0);
        assert_eq!(r.scenarios[0].traces.len(), MAX_TRACES);
        assert!(r.scenarios[0].traces_truncated);
    }

    #[test]
    fn refuses_cycles_unstructured_joins_and_invalid_metrics() {
        let mut d = examples::evaluation_process();
        let a = d.nodes[0].id;
        let b = d.nodes[7].id;
        d.add_flow(b, a).unwrap();
        assert!(evaluate(&d).is_err());
        let mut d = examples::evaluation_process();
        d.add_flow(d.nodes[1].id, d.nodes[3].id).unwrap();
        assert!(evaluate(&d).is_err());
        let mut d = examples::evaluation_process();
        d.nodes[0].cost = f64::NAN;
        assert!(evaluate(&d).is_err());
        d.nodes[0].cost = 0.0;
        d.nodes[0].duration = -1.0;
        assert!(evaluate(&d).is_err());
    }

    #[test]
    fn metrics_persist_in_native_and_bpmn_files_with_legacy_defaults() {
        let mut d = examples::evaluation_process();
        d.nodes[0].cost = 12.5;
        d.nodes[0].duration = 3.25;
        let native = document::to_json(&Model::Bpmn(d.clone()));
        assert_eq!(document::load_any(native.as_bytes()).unwrap(), Model::Bpmn(d.clone()));
        let back = crate::bpmn_xml::import(&crate::bpmn_xml::export(&d)).unwrap();
        let a = back.nodes.iter().find(|n| n.name == "A1").unwrap();
        assert_eq!((a.cost, a.duration), (12.5, 3.25));
        let mut old: serde_json::Value = serde_json::from_str(&native).unwrap();
        for node in old["model"]["model"]["nodes"].as_array_mut().unwrap() {
            node.as_object_mut().unwrap().remove("cost");
            node.as_object_mut().unwrap().remove("duration");
        }
        let Model::Bpmn(old) = document::load_any(old.to_string().as_bytes()).unwrap() else {
            panic!()
        };
        assert!(old.nodes.iter().all(|n| n.cost == 0.0 && n.duration == 0.0));
        assert_eq!(evaluate(&old).unwrap().mean_duration, 0.0);
    }
}
