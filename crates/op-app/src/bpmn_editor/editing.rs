use super::*;
use crate::editor::clipboard::{CanvasClipboard, ClipboardState};

impl CanvasClipboard for BpmnEditor {
    type Fragment = op_core::bpmn::editing::Fragment;
    fn clipboard_state(&mut self) -> &mut ClipboardState {
        &mut self.clipboard
    }
    fn clipboard_kind(&self) -> &'static str {
        "bpmn"
    }
    fn has_selection(&self) -> bool {
        !self.group.is_empty()
            || !self.pool_group.is_empty()
            || matches!(self.selection, Some(Sel::Node(_) | Sel::Pool(_) | Sel::Lane(_, _)))
    }
    fn fragment(&self) -> Self::Fragment {
        let mut nodes = self.group.clone();
        let mut pools = self.pool_group.clone();
        match self.selection {
            Some(Sel::Node(id)) => nodes.push(id),
            Some(Sel::Pool(id)) => pools.push(id),
            Some(Sel::Lane(id, index)) => {
                if let Some(pool) = self.diagram.pool(id)
                    && let Some(lane) = pool.lanes.get(index)
                {
                    let bounds = pool.lane_rect(index);
                    let nodes: Vec<_> = self.diagram.nodes.iter().filter(|n| bounds.contains(n.pos)).map(|n| n.id).collect();
                    let mut fragment = self.diagram.fragment(&nodes, &[]);
                    let mut copy = pool.clone();
                    copy.pos.y = bounds.y;
                    copy.lanes = vec![lane.clone()];
                    fragment.pools.push(copy);
                    return fragment;
                }
            }
            _ => {}
        }
        self.diagram.fragment(&nodes, &pools)
    }
    fn paste_fragment(&mut self, fragment: &Self::Fragment, delta: Pos, commit: bool) {
        if commit {
            self.history.sync(&self.diagram, false);
        } else {
            self.history.begin(&self.diagram);
        }
        match self.diagram.paste(fragment, delta) {
            Ok((nodes, pools)) => {
                self.pool_move_nodes = nodes.clone();
                self.group = nodes;
                self.pool_group = pools.clone();
                self.selection = None;
                if pools.len() == 1
                    && self.group.iter().all(|id| {
                        self.diagram
                            .pool(pools[0])
                            .unwrap()
                            .rect()
                            .contains(self.diagram.node(*id).unwrap().pos)
                    })
                {
                    let pool = pools[0];
                    self.group.clear();
                    self.selection = Some(Sel::Pool(pool));
                } else if pools.is_empty() {
                    self.normalize_group();
                }
                self.tool = Tool::Select;
                self.connect_pending = None;
                self.status.clear();
            }
            Err(error) => self.status = error,
        }
        if commit {
            self.history.sync(&self.diagram, false);
        }
    }
    fn cut_selection(&mut self) {
        self.history.sync(&self.diagram, false);
        let fragment = self.fragment();
        for node in fragment.nodes {
            self.diagram.remove_node(node.id);
        }
        if let Some(Sel::Lane(pool, index)) = self.selection {
            if let Some(pool) = self.diagram.pool_mut(pool)
                && pool.lanes.len() > 1
            {
                pool.lanes.remove(index);
            } else {
                self.diagram.remove_pool(pool);
            }
        } else {
            for pool in fragment.pools {
                self.diagram.remove_pool(pool.id);
            }
        }
        self.group.clear();
        self.pool_group.clear();
        self.selection = None;
        self.history.sync(&self.diagram, false);
    }
    fn select_everything(&mut self) {
        self.select_all();
    }
}
