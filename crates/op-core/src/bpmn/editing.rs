//! Portable BPMN selections; pools include their contained nodes and internal flows.
use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub nodes: Vec<Node>,
    pub flows: Vec<Flow>,
    pub pools: Vec<Pool>,
}
impl BpmnDiagram {
    pub fn fragment(&self, nodes: &[Id], pools: &[Id]) -> Fragment {
        let pools: Vec<_> = self.pools.iter().filter(|p| pools.contains(&p.id)).cloned().collect();
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| nodes.contains(&n.id) || pools.iter().any(|p| p.rect().contains(n.pos)))
            .cloned()
            .collect();
        let ids: HashSet<_> = nodes.iter().map(|n| n.id).collect();
        Fragment {
            flows: self
                .flows
                .iter()
                .filter(|f| ids.contains(&f.from) && ids.contains(&f.to))
                .cloned()
                .collect(),
            nodes,
            pools,
        }
    }
    /// Validate before mutation and remap every identity, including lane identities.
    pub fn paste(&mut self, fragment: &Fragment, delta: Pos) -> Result<(Vec<Id>, Vec<Id>), String> {
        let source = Self {
            nodes: fragment.nodes.clone(),
            flows: fragment.flows.clone(),
            pools: fragment.pools.clone(),
            ..Self::new("")
        };
        source.check_storage()?;
        let mut result = self.clone();
        let mut map = HashMap::new();
        let mut pools = Vec::new();
        for old in &fragment.pools {
            let mut pool = old.clone();
            pool.id = result.fresh_id();
            pool.pos = pool.pos + delta;
            for lane in &mut pool.lanes {
                lane.id = result.fresh_id();
            }
            pools.push(pool.id);
            result.pools.push(pool);
        }
        let mut nodes = Vec::new();
        for old in &fragment.nodes {
            let mut node = old.clone();
            node.id = result.fresh_id();
            node.pos = node.pos + delta;
            map.insert(old.id, node.id);
            nodes.push(node.id);
            result.nodes.push(node);
        }
        for old in &fragment.flows {
            if let (Some(&from), Some(&to)) = (map.get(&old.from), map.get(&old.to)) {
                let mut flow = old.clone();
                flow.id = result.fresh_id();
                flow.from = from;
                flow.to = to;
                result.flows.push(flow);
            }
        }
        result.check_storage()?;
        *self = result;
        Ok((nodes, pools))
    }
}
