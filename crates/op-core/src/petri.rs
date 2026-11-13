//! Place/Transition Petri nets: model, firing rule and incidence matrices.

use crate::geom::Pos;
use crate::i18n::tr;
use serde::{Deserialize, Serialize};

pub type Id = u32;

/// A marking: number of tokens per place, indexed like [`PetriNet::places`].
pub type Marking = Vec<u32>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: Id,
    pub name: String,
    pub tokens: u32,
    /// Optional capacity (place/transition nets with capacities).
    #[serde(default)]
    pub capacity: Option<u32>,
    pub pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub id: Id,
    pub name: String,
    pub pos: Pos,
    /// Silent (τ) transition, e.g. produced by a BPMN gateway conversion.
    #[serde(default)]
    pub silent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRef {
    Place(Id),
    Transition(Id),
}

impl NodeRef {
    pub fn id(self) -> Id {
        match self {
            NodeRef::Place(id) | NodeRef::Transition(id) => id,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcKind {
    #[default]
    Normal,
    /// Inhibitor arc (place → transition only): the transition is enabled only
    /// if the place holds fewer tokens than the arc weight.
    Inhibitor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arc {
    pub id: Id,
    pub from: NodeRef,
    pub to: NodeRef,
    pub weight: u32,
    #[serde(default)]
    pub kind: ArcKind,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PetriNet {
    pub name: String,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
    #[serde(default)]
    pub style: crate::style::DiagramStyle,
    next_id: Id,
}

impl PetriNet {
    /// Analysis input excludes layout and drawing style while preserving all firing rules.
    pub fn analysis_model(&self) -> Self {
        let mut net = self.clone();
        for p in &mut net.places {
            p.pos = Pos::default();
        }
        for t in &mut net.transitions {
            t.pos = Pos::default();
        }
        net.style = Default::default();
        net
    }

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            next_id: 1,
            ..Default::default()
        }
    }

    fn fresh_id(&mut self) -> Id {
        // Robust even if the net was deserialized/imported with explicit ids.
        let max = self
            .places
            .iter()
            .map(|p| p.id)
            .chain(self.transitions.iter().map(|t| t.id))
            .chain(self.arcs.iter().map(|a| a.id))
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max + 1);
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_place(&mut self, pos: Pos, tokens: u32) -> Id {
        let id = self.fresh_id();
        let name = format!("P{}", self.places.len() + 1);
        self.places.push(Place {
            id,
            name,
            tokens,
            capacity: None,
            pos,
        });
        id
    }

    pub fn add_transition(&mut self, pos: Pos) -> Id {
        let id = self.fresh_id();
        let name = format!("T{}", self.transitions.len() + 1);
        self.transitions.push(Transition {
            id,
            name,
            pos,
            silent: false,
        });
        id
    }

    /// Adds an arc. Arcs must connect a place to a transition or vice versa.
    /// If an identical arc already exists, its weight is incremented instead.
    pub fn add_arc(&mut self, from: NodeRef, to: NodeRef) -> Result<Id, &'static str> {
        match (from, to) {
            (NodeRef::Place(_), NodeRef::Transition(_)) | (NodeRef::Transition(_), NodeRef::Place(_)) => {}
            _ => return Err(tr("An arc must connect a place and a transition")),
        }
        if !self.contains(from) || !self.contains(to) {
            return Err(tr("Unknown node"));
        }
        if let Some(a) = self.arcs.iter_mut().find(|a| a.from == from && a.to == to) {
            a.weight += 1;
            return Ok(a.id);
        }
        let id = self.fresh_id();
        self.arcs.push(Arc {
            id,
            from,
            to,
            weight: 1,
            kind: ArcKind::Normal,
        });
        Ok(id)
    }

    pub fn contains(&self, n: NodeRef) -> bool {
        match n {
            NodeRef::Place(id) => self.places.iter().any(|p| p.id == id),
            NodeRef::Transition(id) => self.transitions.iter().any(|t| t.id == id),
        }
    }

    pub fn remove_node(&mut self, n: NodeRef) {
        match n {
            NodeRef::Place(id) => self.places.retain(|p| p.id != id),
            NodeRef::Transition(id) => self.transitions.retain(|t| t.id != id),
        }
        self.arcs.retain(|a| a.from != n && a.to != n);
    }

    pub fn remove_arc(&mut self, id: Id) {
        self.arcs.retain(|a| a.id != id);
    }

    pub fn place(&self, id: Id) -> Option<&Place> {
        self.places.iter().find(|p| p.id == id)
    }

    pub fn place_mut(&mut self, id: Id) -> Option<&mut Place> {
        self.places.iter_mut().find(|p| p.id == id)
    }

    pub fn transition(&self, id: Id) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.id == id)
    }

    pub fn transition_mut(&mut self, id: Id) -> Option<&mut Transition> {
        self.transitions.iter_mut().find(|t| t.id == id)
    }

    pub fn arc(&self, id: Id) -> Option<&Arc> {
        self.arcs.iter().find(|a| a.id == id)
    }

    pub fn arc_mut(&mut self, id: Id) -> Option<&mut Arc> {
        self.arcs.iter_mut().find(|a| a.id == id)
    }

    pub fn place_index(&self, id: Id) -> Option<usize> {
        self.places.iter().position(|p| p.id == id)
    }

    pub fn transition_index(&self, id: Id) -> Option<usize> {
        self.transitions.iter().position(|t| t.id == id)
    }

    pub fn node_pos(&self, n: NodeRef) -> Option<Pos> {
        match n {
            NodeRef::Place(id) => self.place(id).map(|p| p.pos),
            NodeRef::Transition(id) => self.transition(id).map(|t| t.pos),
        }
    }

    pub fn node_name(&self, n: NodeRef) -> Option<&str> {
        match n {
            NodeRef::Place(id) => self.place(id).map(|p| p.name.as_str()),
            NodeRef::Transition(id) => self.transition(id).map(|t| t.name.as_str()),
        }
    }

    pub fn set_node_pos(&mut self, n: NodeRef, pos: Pos) {
        match n {
            NodeRef::Place(id) => {
                if let Some(p) = self.place_mut(id) {
                    p.pos = pos;
                }
            }
            NodeRef::Transition(id) => {
                if let Some(t) = self.transition_mut(id) {
                    t.pos = pos;
                }
            }
        }
    }

    /// Initial marking M0 (tokens stored on the places).
    pub fn initial_marking(&self) -> Marking {
        self.places.iter().map(|p| p.tokens).collect()
    }

    pub fn set_initial_marking(&mut self, m: &[u32]) {
        for (p, &k) in self.places.iter_mut().zip(m) {
            p.tokens = k;
        }
    }

    // ---------------------------------------------------------------------
    // Matrices

    /// Pre (backward incidence) matrix, `pre[p][t]` = weight of arc p → t.
    /// Inhibitor arcs are not part of the incidence matrices.
    pub fn pre_matrix(&self) -> Vec<Vec<u32>> {
        self.matrix(|a| match (a.from, a.to, a.kind) {
            (NodeRef::Place(p), NodeRef::Transition(t), ArcKind::Normal) => Some((p, t)),
            _ => None,
        })
    }

    /// Post (forward incidence) matrix, `post[p][t]` = weight of arc t → p.
    pub fn post_matrix(&self) -> Vec<Vec<u32>> {
        self.matrix(|a| match (a.from, a.to) {
            (NodeRef::Transition(t), NodeRef::Place(p)) => Some((p, t)),
            _ => None,
        })
    }

    fn matrix(&self, sel: impl Fn(&Arc) -> Option<(Id, Id)>) -> Vec<Vec<u32>> {
        let mut m = vec![vec![0u32; self.transitions.len()]; self.places.len()];
        for a in &self.arcs {
            if let Some((p, t)) = sel(a)
                && let (Some(pi), Some(ti)) = (self.place_index(p), self.transition_index(t))
            {
                m[pi][ti] += a.weight;
            }
        }
        m
    }

    /// Incidence matrix C = Post − Pre, `c[p][t]`.
    pub fn incidence_matrix(&self) -> Vec<Vec<i64>> {
        let pre = self.pre_matrix();
        let post = self.post_matrix();
        pre.iter()
            .zip(&post)
            .map(|(r1, r2)| r1.iter().zip(r2).map(|(&a, &b)| b as i64 - a as i64).collect())
            .collect()
    }

    // ---------------------------------------------------------------------
    // Firing rule

    /// Compiled form of the net, convenient for (fast) state-space exploration.
    pub fn compile(&self) -> CompiledNet {
        let nt = self.transitions.len();
        let mut inputs = vec![Vec::new(); nt];
        let mut outputs = vec![Vec::new(); nt];
        let mut inhibitors = vec![Vec::new(); nt];
        for a in &self.arcs {
            match (a.from, a.to) {
                (NodeRef::Place(p), NodeRef::Transition(t)) => {
                    let (Some(pi), Some(ti)) = (self.place_index(p), self.transition_index(t)) else {
                        continue;
                    };
                    match a.kind {
                        ArcKind::Normal => inputs[ti].push((pi, a.weight)),
                        ArcKind::Inhibitor => inhibitors[ti].push((pi, a.weight)),
                    }
                }
                (NodeRef::Transition(t), NodeRef::Place(p)) => {
                    let (Some(pi), Some(ti)) = (self.place_index(p), self.transition_index(t)) else {
                        continue;
                    };
                    outputs[ti].push((pi, a.weight));
                }
                _ => {}
            }
        }
        CompiledNet {
            inputs,
            outputs,
            inhibitors,
            capacities: self.places.iter().map(|p| p.capacity).collect(),
        }
    }

    pub fn is_enabled(&self, t_index: usize, m: &[u32]) -> bool {
        self.compile().is_enabled(t_index, m)
    }

    pub fn enabled_transitions(&self, m: &[u32]) -> Vec<usize> {
        let c = self.compile();
        (0..self.transitions.len()).filter(|&t| c.is_enabled(t, m)).collect()
    }

    /// Fires transition `t_index` from marking `m`, returning the new marking,
    /// or `None` if it is not enabled.
    pub fn fire(&self, t_index: usize, m: &[u32]) -> Option<Marking> {
        self.compile().fire(t_index, m)
    }
}

/// Special value representing ω (unbounded) in a coverability marking.
pub const OMEGA: u64 = u64::MAX;

/// Petri net reduced to index-based arc lists.
#[derive(Clone, Debug)]
pub struct CompiledNet {
    pub inputs: Vec<Vec<(usize, u32)>>,
    pub outputs: Vec<Vec<(usize, u32)>>,
    pub inhibitors: Vec<Vec<(usize, u32)>>,
    pub capacities: Vec<Option<u32>>,
}

impl CompiledNet {
    pub fn transition_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn is_enabled(&self, t: usize, m: &[u32]) -> bool {
        let w: Vec<u64> = m.iter().map(|&x| x as u64).collect();
        self.is_enabled_omega(t, &w)
    }

    pub fn fire(&self, t: usize, m: &[u32]) -> Option<Marking> {
        let w: Vec<u64> = m.iter().map(|&x| x as u64).collect();
        self.fire_omega(t, &w)
            .map(|r| r.into_iter().map(|x| x.min(u32::MAX as u64) as u32).collect())
    }

    /// Enabling check on markings that may contain ω.
    pub fn is_enabled_omega(&self, t: usize, m: &[u64]) -> bool {
        self.inputs[t].iter().all(|&(p, w)| m[p] >= w as u64)
            && self.inhibitors[t].iter().all(|&(p, w)| m[p] != OMEGA && m[p] < w as u64)
            && self.check_capacity(t, m)
    }

    fn check_capacity(&self, t: usize, m: &[u64]) -> bool {
        // Capacity check on the resulting marking (strict capacity semantics).
        for &(p, w_out) in &self.outputs[t] {
            if let Some(cap) = self.capacities[p] {
                if m[p] == OMEGA {
                    continue;
                }
                let consumed: u64 = self.inputs[t].iter().filter(|&&(q, _)| q == p).map(|&(_, w)| w as u64).sum();
                if m[p].saturating_sub(consumed) + w_out as u64 > cap as u64 {
                    return false;
                }
            }
        }
        true
    }

    pub fn fire_omega(&self, t: usize, m: &[u64]) -> Option<Vec<u64>> {
        if !self.is_enabled_omega(t, m) {
            return None;
        }
        let mut r = m.to_vec();
        for &(p, w) in &self.inputs[t] {
            if r[p] != OMEGA {
                r[p] -= w as u64;
            }
        }
        for &(p, w) in &self.outputs[t] {
            if r[p] != OMEGA {
                r[p] = r[p].saturating_add(w as u64).min(OMEGA - 1);
            }
        }
        Some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// p1 --t1--> p2 --t2--> p1
    pub fn cycle() -> PetriNet {
        let mut n = PetriNet::new("cycle");
        let p1 = n.add_place(Pos::new(0.0, 0.0), 1);
        let p2 = n.add_place(Pos::new(100.0, 0.0), 0);
        let t1 = n.add_transition(Pos::new(50.0, -30.0));
        let t2 = n.add_transition(Pos::new(50.0, 30.0));
        n.add_arc(NodeRef::Place(p1), NodeRef::Transition(t1)).unwrap();
        n.add_arc(NodeRef::Transition(t1), NodeRef::Place(p2)).unwrap();
        n.add_arc(NodeRef::Place(p2), NodeRef::Transition(t2)).unwrap();
        n.add_arc(NodeRef::Transition(t2), NodeRef::Place(p1)).unwrap();
        n
    }

    #[test]
    fn matrices() {
        let n = cycle();
        assert_eq!(n.pre_matrix(), vec![vec![1, 0], vec![0, 1]]);
        assert_eq!(n.post_matrix(), vec![vec![0, 1], vec![1, 0]]);
        assert_eq!(n.incidence_matrix(), vec![vec![-1, 1], vec![1, -1]]);
    }

    #[test]
    fn firing() {
        let n = cycle();
        let m0 = n.initial_marking();
        assert_eq!(n.enabled_transitions(&m0), vec![0]);
        let m1 = n.fire(0, &m0).unwrap();
        assert_eq!(m1, vec![0, 1]);
        assert!(n.fire(0, &m1).is_none());
    }

    #[test]
    fn invalid_arc() {
        let mut n = PetriNet::new("x");
        let a = n.add_place(Pos::default(), 0);
        let b = n.add_place(Pos::default(), 0);
        assert!(n.add_arc(NodeRef::Place(a), NodeRef::Place(b)).is_err());
    }

    #[test]
    fn arc_weight_increments() {
        let mut n = cycle();
        let p1 = n.places[0].id;
        let t1 = n.transitions[0].id;
        n.add_arc(NodeRef::Place(p1), NodeRef::Transition(t1)).unwrap();
        assert_eq!(n.pre_matrix()[0][0], 2);
    }

    #[test]
    fn inhibitor() {
        let mut n = cycle();
        let p2 = n.places[1].id;
        let t1 = n.transitions[0].id;
        let id = n.add_arc(NodeRef::Place(p2), NodeRef::Transition(t1)).unwrap();
        n.arc_mut(id).unwrap().kind = ArcKind::Inhibitor;
        assert!(n.is_enabled(0, &[1, 0]));
        assert!(!n.is_enabled(0, &[1, 1]));
    }
}
