//! Behavioural and structural analysis of Petri nets:
//! reachability / coverability graph (Karp–Miller), boundedness, liveness,
//! reversibility, deadlocks, P/T-invariants (Farkas), structural classes
//! and workflow-net soundness.

use serde::{Deserialize, Serialize};

use crate::i18n::{tr, trf};
use crate::petri::{ArcKind, NodeRef, OMEGA, PetriNet};
use std::collections::{HashMap, VecDeque};

/// Reachability graph, or coverability graph when the net is unbounded
/// (markings may then contain [`OMEGA`]).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StateSpace {
    pub states: Vec<Vec<u64>>,
    /// `(from_state, transition_index, to_state)`
    pub edges: Vec<(usize, usize, usize)>,
    /// BFS depth of every state (useful for layout).
    pub depth: Vec<usize>,
    /// `expanded[i]` is false when exploration stopped before state `i` was explored.
    pub expanded: Vec<bool>,
    /// True if the exploration hit the state limit.
    pub truncated: bool,
}

impl StateSpace {
    /// Builds the reachability (or Karp–Miller coverability) graph, exploring
    /// at most `limit` states.
    pub fn explore(net: &PetriNet, limit: usize) -> StateSpace {
        Self::explore_cancellable(net, limit, &|| false)
    }

    pub fn explore_cancellable(net: &PetriNet, limit: usize, cancelled: &impl Fn() -> bool) -> StateSpace {
        let c = net.compile();
        let m0: Vec<u64> = net.initial_marking().into_iter().map(u64::from).collect();
        let mut ss = StateSpace::default();
        let mut index: HashMap<Vec<u64>, usize> = HashMap::new();
        let mut parent: Vec<Option<usize>> = vec![None];
        index.insert(m0.clone(), 0);
        ss.states.push(m0);
        ss.depth.push(0);
        ss.expanded.push(false);
        let mut queue = VecDeque::from([0usize]);
        // Karp–Miller acceleration is only valid for monotonic nets
        // (no inhibitor arcs, no capacities).
        let monotonic = c.inhibitors.iter().all(|v| v.is_empty()) && c.capacities.iter().all(|c| c.is_none());

        while let Some(i) = queue.pop_front() {
            if cancelled() {
                ss.truncated = true;
                break;
            }
            ss.expanded[i] = true;
            for t in 0..c.transition_count() {
                if cancelled() {
                    ss.truncated = true;
                    return ss;
                }
                let Some(mut m2) = c.fire_omega(t, &ss.states[i]) else { continue };
                // Karp–Miller acceleration: if m2 strictly covers an ancestor,
                // every strictly increased place becomes ω.
                let mut a = if monotonic { Some(i) } else { None };
                while let Some(k) = a {
                    let anc = &ss.states[k];
                    if anc.iter().zip(&m2).all(|(x, y)| x <= y) && *anc != m2 {
                        for (p, v) in m2.iter_mut().enumerate() {
                            if anc[p] < *v {
                                *v = OMEGA;
                            }
                        }
                    }
                    a = parent[k];
                }
                let j = match index.get(&m2) {
                    Some(&j) => j,
                    None => {
                        if ss.states.len() >= limit {
                            ss.truncated = true;
                            continue;
                        }
                        let j = ss.states.len();
                        index.insert(m2.clone(), j);
                        ss.states.push(m2);
                        ss.depth.push(ss.depth[i] + 1);
                        ss.expanded.push(false);
                        parent.push(Some(i));
                        queue.push_back(j);
                        j
                    }
                };
                ss.edges.push((i, t, j));
            }
        }
        ss
    }

    pub fn has_omega(&self) -> bool {
        self.states.iter().any(|m| m.contains(&OMEGA))
    }

    pub fn is_complete(&self) -> bool {
        !self.truncated
    }

    /// Strongly connected components (Tarjan, iterative). Returns the
    /// component id of each state, and the number of components.
    pub fn scc(&self) -> (Vec<usize>, usize) {
        let n = self.states.len();
        let mut adj = vec![Vec::new(); n];
        for &(a, _, b) in &self.edges {
            adj[a].push(b);
        }
        let mut index = vec![usize::MAX; n];
        let mut low = vec![0; n];
        let mut on_stack = vec![false; n];
        let mut stack = Vec::new();
        let mut comp = vec![usize::MAX; n];
        let mut next_index = 0;
        let mut ncomp = 0;

        for root in 0..n {
            if index[root] != usize::MAX {
                continue;
            }
            // (node, next child position)
            let mut call: Vec<(usize, usize)> = vec![(root, 0)];
            index[root] = next_index;
            low[root] = next_index;
            next_index += 1;
            stack.push(root);
            on_stack[root] = true;
            while let Some(&mut (v, ref mut ci)) = call.last_mut() {
                if *ci < adj[v].len() {
                    let w = adj[v][*ci];
                    *ci += 1;
                    if index[w] == usize::MAX {
                        index[w] = next_index;
                        low[w] = next_index;
                        next_index += 1;
                        stack.push(w);
                        on_stack[w] = true;
                        call.push((w, 0));
                    } else if on_stack[w] {
                        low[v] = low[v].min(index[w]);
                    }
                } else {
                    call.pop();
                    if let Some(&(u, _)) = call.last() {
                        low[u] = low[u].min(low[v]);
                    }
                    if low[v] == index[v] {
                        while let Some(w) = stack.pop() {
                            on_stack[w] = false;
                            comp[w] = ncomp;
                            if w == v {
                                break;
                            }
                        }
                        ncomp += 1;
                    }
                }
            }
        }
        (comp, ncomp)
    }

    /// Shortest firing sequence (transition indices) from M0 to `target`.
    pub fn path_to(&self, target: usize) -> Option<Vec<usize>> {
        let n = self.states.len();
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; n];
        let mut seen = vec![false; n];
        seen[0] = true;
        let mut q = VecDeque::from([0usize]);
        let mut adj = vec![Vec::new(); n];
        for &(a, t, b) in &self.edges {
            adj[a].push((t, b));
        }
        while let Some(v) = q.pop_front() {
            if v == target {
                let mut path = Vec::new();
                let mut cur = v;
                while let Some((p, t)) = prev[cur] {
                    path.push(t);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            for &(t, w) in &adj[v] {
                if !seen[w] {
                    seen[w] = true;
                    prev[w] = Some((v, t));
                    q.push_back(w);
                }
            }
        }
        None
    }
}

/// Formats a marking, using `ω` for unbounded places.
pub fn format_marking(m: &[u64]) -> String {
    let parts: Vec<String> = m
        .iter()
        .map(|&v| if v == OMEGA { "ω".to_string() } else { v.to_string() })
        .collect();
    format!("({})", parts.join(", "))
}

/// Behavioural properties computed from the state space.
/// `None` means "could not be decided" (state space truncated or unbounded).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BehaviouralReport {
    pub state_count: usize,
    pub edge_count: usize,
    pub truncated: bool,
    pub bounded: Option<bool>,
    /// Bound of every place (`None` = ω / unbounded).
    pub place_bounds: Vec<Option<u64>>,
    pub safe: Option<bool>,
    /// Reachable dead markings (state indices).
    pub deadlocks: Vec<usize>,
    pub deadlock_free: Option<bool>,
    /// Transition can fire at least once (L1-live / quasi-live).
    pub quasi_live: Vec<bool>,
    /// Transition is live (L4): fireable from every reachable marking.
    pub live_transitions: Option<Vec<bool>>,
    pub live: Option<bool>,
    /// M0 is reachable from every reachable marking.
    pub reversible: Option<bool>,
}

impl BehaviouralReport {
    pub fn compute(net: &PetriNet, ss: &StateSpace) -> Self {
        let np = net.places.len();
        let nt = net.transitions.len();
        let omega = ss.has_omega();
        let complete = ss.is_complete();

        let mut place_bounds = vec![Some(0u64); np];
        for m in &ss.states {
            for (p, &v) in m.iter().enumerate() {
                place_bounds[p] = match place_bounds[p] {
                    None => None,
                    Some(_) if v == OMEGA => None,
                    Some(b) => Some(b.max(v)),
                };
            }
        }
        let bounded = if omega {
            Some(false)
        } else if complete {
            Some(true)
        } else {
            None
        };
        let safe = match bounded {
            Some(false) => Some(false),
            _ if place_bounds.iter().any(|b| b.is_some_and(|b| b > 1)) => Some(false),
            Some(true) => Some(true),
            None => None,
        };

        let mut out_deg = vec![0usize; ss.states.len()];
        let mut quasi_live = vec![false; nt];
        for &(a, t, _) in &ss.edges {
            out_deg[a] += 1;
            quasi_live[t] = true;
        }
        let deadlocks: Vec<usize> = (0..ss.states.len())
            .filter(|&i| ss.expanded[i] && out_deg[i] == 0 && !ss.states[i].contains(&OMEGA))
            .collect();
        let deadlock_free = if !deadlocks.is_empty() {
            Some(false)
        } else if complete && !omega {
            Some(true)
        } else {
            None
        };

        let (mut live_transitions, mut live, mut reversible) = (None, None, None);
        if complete && !omega {
            let (comp, ncomp) = ss.scc();
            // A component is terminal if no edge leaves it.
            let mut terminal = vec![true; ncomp];
            let mut comp_trans = vec![vec![false; nt]; ncomp];
            for &(a, t, b) in &ss.edges {
                if comp[a] != comp[b] {
                    terminal[comp[a]] = false;
                } else {
                    comp_trans[comp[a]][t] = true;
                }
            }
            let lt: Vec<bool> = (0..nt)
                .map(|t| (0..ncomp).filter(|&c| terminal[c]).all(|c| comp_trans[c][t]))
                .collect();
            live = Some(lt.iter().all(|&x| x));
            live_transitions = Some(lt);
            reversible = Some(ncomp == 1 || comp.iter().all(|&c| c == comp[0]));
        }

        BehaviouralReport {
            state_count: ss.states.len(),
            edge_count: ss.edges.len(),
            truncated: ss.truncated,
            bounded,
            place_bounds,
            safe,
            deadlocks,
            deadlock_free,
            quasi_live,
            live_transitions,
            live,
            reversible,
        }
    }
}

// -------------------------------------------------------------------------
// Invariants

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn normalize(v: &mut [i64]) {
    let g = v.iter().fold(0, |g, &x| gcd(g, x));
    if g > 1 {
        v.iter_mut().for_each(|x| *x /= g);
    }
}

/// Minimal semi-positive solutions `y ≥ 0, y ≠ 0` of `yᵀ·A = 0`
/// (Farkas / Martínez–Silva algorithm). `a` is `rows × cols`.
/// Returns `None` if the computation blows up past `max_rows`.
pub fn farkas(a: &[Vec<i64>], max_rows: usize) -> Option<Vec<Vec<i64>>> {
    let n = a.len();
    if n == 0 {
        return Some(Vec::new());
    }
    let m = a[0].len();
    // Each row: (A-part, identity-part)
    let mut rows: Vec<(Vec<i64>, Vec<i64>)> = (0..n)
        .map(|i| {
            let mut id = vec![0; n];
            id[i] = 1;
            (a[i].clone(), id)
        })
        .collect();

    for j in 0..m {
        let mut next: Vec<(Vec<i64>, Vec<i64>)> = rows.iter().filter(|r| r.0[j] == 0).cloned().collect();
        let pos: Vec<&(Vec<i64>, Vec<i64>)> = rows.iter().filter(|r| r.0[j] > 0).collect();
        let neg: Vec<&(Vec<i64>, Vec<i64>)> = rows.iter().filter(|r| r.0[j] < 0).collect();
        for r1 in &pos {
            for r2 in &neg {
                let (k1, k2) = (-r2.0[j], r1.0[j]);
                let mut av: Vec<i64> = r1.0.iter().zip(&r2.0).map(|(x, y)| k1 * x + k2 * y).collect();
                let mut iv: Vec<i64> = r1.1.iter().zip(&r2.1).map(|(x, y)| k1 * x + k2 * y).collect();
                let g = av.iter().chain(iv.iter()).fold(0, |g, &x| gcd(g, x));
                if g > 1 {
                    av.iter_mut().for_each(|x| *x /= g);
                    iv.iter_mut().for_each(|x| *x /= g);
                }
                next.push((av, iv));
            }
        }
        rows = remove_non_minimal(next);
        if rows.len() > max_rows {
            return None;
        }
    }
    let mut res: Vec<Vec<i64>> = rows.into_iter().map(|r| r.1).collect();
    for v in &mut res {
        normalize(v);
    }
    res.sort();
    res.dedup();
    Some(res)
}

fn remove_non_minimal(rows: Vec<(Vec<i64>, Vec<i64>)>) -> Vec<(Vec<i64>, Vec<i64>)> {
    let supp: Vec<Vec<bool>> = rows.iter().map(|r| r.1.iter().map(|&x| x != 0).collect()).collect();
    let mut keep = vec![true; rows.len()];
    for i in 0..rows.len() {
        for k in 0..rows.len() {
            if i == k || !keep[k] {
                continue;
            }
            // supp(k) ⊆ supp(i) → i is not minimal (keep the first of equal supports)
            let subset = supp[k].iter().zip(&supp[i]).all(|(&a, &b)| !a || b);
            let equal = supp[k] == supp[i];
            if subset && (!equal || k < i) {
                keep[i] = false;
                break;
            }
        }
    }
    rows.into_iter().zip(keep).filter(|(_, k)| *k).map(|(r, _)| r).collect()
}

fn transpose(a: &[Vec<i64>], rows: usize, cols: usize) -> Vec<Vec<i64>> {
    (0..cols).map(|j| (0..rows).map(|i| a[i][j]).collect()).collect()
}

/// Minimal semi-positive P-invariants (vectors indexed by place).
pub fn p_invariants(net: &PetriNet) -> Option<Vec<Vec<i64>>> {
    farkas(&net.incidence_matrix(), 2000)
}

/// Minimal semi-positive T-invariants (vectors indexed by transition).
pub fn t_invariants(net: &PetriNet) -> Option<Vec<Vec<i64>>> {
    let c = net.incidence_matrix();
    let ct = transpose(&c, net.places.len(), net.transitions.len());
    farkas(&ct, 2000)
}

/// True if every index is in the support of at least one invariant.
pub fn covered(invariants: &[Vec<i64>], len: usize) -> bool {
    (0..len).all(|i| invariants.iter().any(|v| v[i] != 0))
}

// -------------------------------------------------------------------------
// Structure

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StructuralReport {
    /// All arc weights are 1.
    pub ordinary: bool,
    /// No place is both input and output of the same transition.
    pub pure: bool,
    /// Every transition has exactly one input and one output place.
    pub state_machine: bool,
    /// Every place has exactly one input and one output transition.
    pub marked_graph: bool,
    pub free_choice: bool,
    pub extended_free_choice: bool,
    pub source_places: Vec<usize>,
    pub sink_places: Vec<usize>,
    pub source_transitions: Vec<usize>,
    pub sink_transitions: Vec<usize>,
    /// One source place, one sink place, every node on a path between them.
    pub workflow_net: bool,
    pub has_inhibitor_arcs: bool,
}

impl StructuralReport {
    pub fn compute(net: &PetriNet) -> Self {
        let np = net.places.len();
        let nt = net.transitions.len();
        let pre = net.pre_matrix();
        let post = net.post_matrix();
        let mut r = StructuralReport {
            ordinary: net.arcs.iter().all(|a| a.weight == 1),
            has_inhibitor_arcs: net.arcs.iter().any(|a| a.kind == ArcKind::Inhibitor),
            ..Default::default()
        };
        r.pure = (0..np).all(|p| (0..nt).all(|t| pre[p][t] == 0 || post[p][t] == 0));
        let t_in = |t: usize| (0..np).filter(|&p| pre[p][t] > 0).count();
        let t_out = |t: usize| (0..np).filter(|&p| post[p][t] > 0).count();
        let p_in = |p: usize| (0..nt).filter(|&t| post[p][t] > 0).count();
        let p_out = |p: usize| (0..nt).filter(|&t| pre[p][t] > 0).count();
        r.state_machine = nt > 0 && (0..nt).all(|t| t_in(t) == 1 && t_out(t) == 1);
        r.marked_graph = np > 0 && (0..np).all(|p| p_in(p) == 1 && p_out(p) == 1);

        // Free choice: if p has several output transitions, each of them has p as only input.
        r.free_choice = (0..np).all(|p| {
            let outs: Vec<usize> = (0..nt).filter(|&t| pre[p][t] > 0).collect();
            outs.len() <= 1 || outs.iter().all(|&t| t_in(t) == 1)
        });
        // Extended free choice: places sharing an output transition share all of them.
        let postset = |p: usize| -> Vec<bool> { (0..nt).map(|t| pre[p][t] > 0).collect() };
        r.extended_free_choice = (0..np).all(|p1| {
            (0..np).all(|p2| {
                let (a, b) = (postset(p1), postset(p2));
                let share = a.iter().zip(&b).any(|(&x, &y)| x && y);
                !share || a == b
            })
        });

        r.source_places = (0..np).filter(|&p| p_in(p) == 0).collect();
        r.sink_places = (0..np).filter(|&p| p_out(p) == 0).collect();
        r.source_transitions = (0..nt).filter(|&t| t_in(t) == 0).collect();
        r.sink_transitions = (0..nt).filter(|&t| t_out(t) == 0).collect();

        r.workflow_net = r.source_places.len() == 1 && r.sink_places.len() == 1 && {
            // every node reachable from i and co-reachable to o
            let n = np + nt;
            let mut adj = vec![Vec::new(); n];
            let mut radj = vec![Vec::new(); n];
            for p in 0..np {
                for t in 0..nt {
                    if pre[p][t] > 0 {
                        adj[p].push(np + t);
                        radj[np + t].push(p);
                    }
                    if post[p][t] > 0 {
                        adj[np + t].push(p);
                        radj[p].push(np + t);
                    }
                }
            }
            let reach = |start: usize, g: &Vec<Vec<usize>>| {
                let mut seen = vec![false; n];
                let mut st = vec![start];
                seen[start] = true;
                while let Some(v) = st.pop() {
                    for &w in &g[v] {
                        if !seen[w] {
                            seen[w] = true;
                            st.push(w);
                        }
                    }
                }
                seen
            };
            let f = reach(r.source_places[0], &adj);
            let b = reach(r.sink_places[0], &radj);
            f.iter().zip(&b).all(|(&x, &y)| x && y)
        };
        r
    }
}

/// Result of the workflow-net soundness check (van der Aalst).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Soundness {
    NotWorkflowNet,
    /// Could not be decided within the state limit.
    Unknown,
    Sound,
    Unsound(Vec<String>),
}

/// A WF-net is sound iff its short-circuited net (with an extra transition
/// o → i), started with one token in i, is live and bounded.
pub fn soundness(net: &PetriNet, limit: usize) -> Soundness {
    soundness_cancellable(net, limit, &|| false)
}

pub fn soundness_cancellable(net: &PetriNet, limit: usize, cancelled: &impl Fn() -> bool) -> Soundness {
    let s = StructuralReport::compute(net);
    if !s.workflow_net {
        return Soundness::NotWorkflowNet;
    }
    let mut sc = net.clone();
    for p in &mut sc.places {
        p.tokens = 0;
    }
    let i = sc.places[s.source_places[0]].id;
    let o = sc.places[s.sink_places[0]].id;
    sc.place_mut(i).unwrap().tokens = 1;
    let t = sc.add_transition(Default::default());
    sc.add_arc(NodeRef::Place(o), NodeRef::Transition(t)).unwrap();
    sc.add_arc(NodeRef::Transition(t), NodeRef::Place(i)).unwrap();
    let ss = StateSpace::explore_cancellable(&sc, limit, cancelled);
    if cancelled() {
        return Soundness::Unknown;
    }
    let rep = BehaviouralReport::compute(&sc, &ss);
    let mut problems = Vec::new();
    if rep.bounded == Some(false) {
        problems.push(tr("analysis.short_circuited_net_unbounded").to_string());
    }
    if let Some(lt) = &rep.live_transitions {
        for (k, l) in lt.iter().enumerate().take(net.transitions.len()) {
            if !l {
                problems.push(trf(
                    "analysis.transition_not_live_dead_may_never_complete",
                    &[&(net.transitions[k].name)],
                ));
            }
        }
    }
    if !problems.is_empty() {
        Soundness::Unsound(problems)
    } else if rep.live == Some(true) && rep.bounded == Some(true) {
        Soundness::Sound
    } else {
        Soundness::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Pos;

    fn producer_consumer(unbounded: bool) -> PetriNet {
        // p1 <-> t1 (loop) producing into p2 ; t2 consumes p2
        let mut n = PetriNet::new("pc");
        let p1 = n.add_place(Pos::default(), 1);
        let p2 = n.add_place(Pos::default(), 0);
        let t1 = n.add_transition(Pos::default());
        let t2 = n.add_transition(Pos::default());
        n.add_arc(NodeRef::Place(p1), NodeRef::Transition(t1)).unwrap();
        n.add_arc(NodeRef::Transition(t1), NodeRef::Place(p1)).unwrap();
        n.add_arc(NodeRef::Transition(t1), NodeRef::Place(p2)).unwrap();
        n.add_arc(NodeRef::Place(p2), NodeRef::Transition(t2)).unwrap();
        if !unbounded {
            n.place_mut(p2).unwrap().capacity = Some(2);
        }
        n
    }

    fn mutex() -> PetriNet {
        // Two processes idle_i -> crit_i (needs mutex) -> idle_i
        let mut n = PetriNet::new("mutex");
        let mtx = n.add_place(Pos::default(), 1);
        for _ in 0..2 {
            let idle = n.add_place(Pos::default(), 1);
            let crit = n.add_place(Pos::default(), 0);
            let enter = n.add_transition(Pos::default());
            let leave = n.add_transition(Pos::default());
            for (a, b) in [
                (NodeRef::Place(idle), NodeRef::Transition(enter)),
                (NodeRef::Place(mtx), NodeRef::Transition(enter)),
                (NodeRef::Transition(enter), NodeRef::Place(crit)),
                (NodeRef::Place(crit), NodeRef::Transition(leave)),
                (NodeRef::Transition(leave), NodeRef::Place(idle)),
                (NodeRef::Transition(leave), NodeRef::Place(mtx)),
            ] {
                n.add_arc(a, b).unwrap();
            }
        }
        n
    }

    #[test]
    fn mutex_properties() {
        let n = mutex();
        let ss = StateSpace::explore(&n, 1000);
        assert_eq!(ss.states.len(), 3);
        let r = BehaviouralReport::compute(&n, &ss);
        assert_eq!(r.bounded, Some(true));
        assert_eq!(r.safe, Some(true));
        assert_eq!(r.live, Some(true));
        assert_eq!(r.reversible, Some(true));
        assert_eq!(r.deadlock_free, Some(true));

        let pinv = p_invariants(&n).unwrap();
        // mutex + crit1 + crit2 ; idle1 + crit1 ; idle2 + crit2
        assert_eq!(pinv.len(), 3);
        assert!(covered(&pinv, n.places.len()));
        let tinv = t_invariants(&n).unwrap();
        assert_eq!(tinv.len(), 2);
    }

    #[test]
    fn unbounded_detected() {
        let n = producer_consumer(true);
        let ss = StateSpace::explore(&n, 1000);
        assert!(ss.has_omega());
        let r = BehaviouralReport::compute(&n, &ss);
        assert_eq!(r.bounded, Some(false));
        assert_eq!(r.place_bounds[1], None);
        assert_eq!(r.place_bounds[0], Some(1));
    }

    #[test]
    fn capacity_bounds() {
        let n = producer_consumer(false);
        let ss = StateSpace::explore(&n, 1000);
        let r = BehaviouralReport::compute(&n, &ss);
        assert_eq!(r.bounded, Some(true));
        assert_eq!(r.place_bounds[1], Some(2));
    }

    #[test]
    fn deadlock() {
        let mut n = PetriNet::new("d");
        let p = n.add_place(Pos::default(), 1);
        let q = n.add_place(Pos::default(), 0);
        let t = n.add_transition(Pos::default());
        n.add_arc(NodeRef::Place(p), NodeRef::Transition(t)).unwrap();
        n.add_arc(NodeRef::Transition(t), NodeRef::Place(q)).unwrap();
        let ss = StateSpace::explore(&n, 100);
        let r = BehaviouralReport::compute(&n, &ss);
        assert_eq!(r.deadlocks, vec![1]);
        assert_eq!(r.live, Some(false));
        assert_eq!(ss.path_to(1), Some(vec![0]));
        let s = StructuralReport::compute(&n);
        assert!(s.workflow_net && s.state_machine);
        assert!(matches!(soundness(&n, 100), Soundness::Sound));
    }

    #[test]
    fn unsound_wf_net() {
        // i -> t1 -> (a, b) ; a -> t2 -> o  (b never consumed: unbounded once short-circuited)
        let mut n = PetriNet::new("u");
        let i = n.add_place(Pos::default(), 1);
        let a = n.add_place(Pos::default(), 0);
        let b = n.add_place(Pos::default(), 0);
        let o = n.add_place(Pos::default(), 0);
        let t1 = n.add_transition(Pos::default());
        let t2 = n.add_transition(Pos::default());
        let t3 = n.add_transition(Pos::default());
        for (x, y) in [
            (NodeRef::Place(i), NodeRef::Transition(t1)),
            (NodeRef::Transition(t1), NodeRef::Place(a)),
            (NodeRef::Transition(t1), NodeRef::Place(b)),
            (NodeRef::Place(a), NodeRef::Transition(t2)),
            (NodeRef::Transition(t2), NodeRef::Place(o)),
            (NodeRef::Place(b), NodeRef::Transition(t3)),
            (NodeRef::Place(a), NodeRef::Transition(t3)),
            (NodeRef::Transition(t3), NodeRef::Place(o)),
        ] {
            n.add_arc(x, y).unwrap();
        }
        assert!(StructuralReport::compute(&n).workflow_net);
        assert!(matches!(soundness(&n, 1000), Soundness::Unsound(_)));
    }
}
