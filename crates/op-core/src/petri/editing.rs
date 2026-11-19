//! Typed Petri selections preserve markings, capacities and arc semantics.
use super::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
}
impl PetriNet {
    pub fn fragment(&self, nodes: &[NodeRef]) -> Fragment {
        Fragment {
            places: self
                .places
                .iter()
                .filter(|p| nodes.contains(&NodeRef::Place(p.id)))
                .cloned()
                .collect(),
            transitions: self
                .transitions
                .iter()
                .filter(|t| nodes.contains(&NodeRef::Transition(t.id)))
                .cloned()
                .collect(),
            arcs: self
                .arcs
                .iter()
                .filter(|a| nodes.contains(&a.from) && nodes.contains(&a.to))
                .cloned()
                .collect(),
        }
    }
    pub fn paste(&mut self, fragment: &Fragment, delta: Pos) -> Result<Vec<NodeRef>, String> {
        Self {
            places: fragment.places.clone(),
            transitions: fragment.transitions.clone(),
            arcs: fragment.arcs.clone(),
            ..Self::new("")
        }
        .check_storage()?;
        let mut result = self.clone();
        let mut map = HashMap::new();
        let mut nodes = Vec::new();
        for old in &fragment.places {
            let mut place = old.clone();
            place.id = result.fresh_id();
            place.pos = place.pos + delta;
            let node = NodeRef::Place(place.id);
            map.insert(NodeRef::Place(old.id), node);
            nodes.push(node);
            result.places.push(place);
        }
        for old in &fragment.transitions {
            let mut transition = old.clone();
            transition.id = result.fresh_id();
            transition.pos = transition.pos + delta;
            let node = NodeRef::Transition(transition.id);
            map.insert(NodeRef::Transition(old.id), node);
            nodes.push(node);
            result.transitions.push(transition);
        }
        for old in &fragment.arcs {
            if let (Some(&from), Some(&to)) = (map.get(&old.from), map.get(&old.to)) {
                let mut arc = old.clone();
                arc.id = result.fresh_id();
                arc.from = from;
                arc.to = to;
                result.arcs.push(arc);
            }
        }
        result.check_storage()?;
        *self = result;
        Ok(nodes)
    }
}
