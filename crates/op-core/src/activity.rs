//! Initial UML activity subset: actions, decisions/merges, fork/join and final nodes.
use crate::flow::{Diagram, FlowKind, Shape};
use crate::geom::Pos;
use crate::i18n::{key, tr};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Initial,
    Action,
    Decision,
    Merge,
    Fork,
    Join,
    Final,
    FlowFinal,
}
pub type ActivityDiagram = Diagram<ActivityKind>;
impl FlowKind for ActivityKind {
    fn all() -> &'static [Self] {
        &[
            Self::Initial,
            Self::Action,
            Self::Decision,
            Self::Merge,
            Self::Fork,
            Self::Join,
            Self::Final,
            Self::FlowFinal,
        ]
    }
    fn label(self) -> &'static str {
        match self {
            Self::Initial => key("activity.initial_node"),
            Self::Action => key("activity.action"),
            Self::Decision => key("activity.decision"),
            Self::Merge => key("activity.merge"),
            Self::Fork => key("activity.fork"),
            Self::Join => key("activity.join"),
            Self::Final => key("activity.activity_final"),
            Self::FlowFinal => key("activity.flow_final"),
        }
    }
    fn shape(self) -> Shape {
        match self {
            Self::Initial => Shape::Initial,
            Self::Action => Shape::Action,
            Self::Decision | Self::Merge => Shape::Diamond,
            Self::Fork | Self::Join => Shape::Bar,
            Self::Final => Shape::Final,
            Self::FlowFinal => Shape::FlowFinal,
        }
    }
    fn can_connect(self, to: Self) -> bool {
        !matches!(self, Self::Final | Self::FlowFinal) && to != Self::Initial
    }
    fn degree_valid(self, i: usize, o: usize) -> bool {
        match self {
            Self::Initial => i == 0 && o == 1,
            Self::Final | Self::FlowFinal => i >= 1 && o == 0,
            Self::Action => i >= 1 && o >= 1,
            Self::Decision | Self::Fork => i == 1 && o >= 2,
            Self::Merge | Self::Join => i >= 2 && o == 1,
        }
    }
}
pub fn example() -> ActivityDiagram {
    let mut d = ActivityDiagram::new(tr("activity.order_approval"));
    let a = d.add_node(ActivityKind::Initial, Pos::new(70.0, 160.0), "");
    let b = d.add_node(ActivityKind::Action, Pos::new(230.0, 160.0), tr("activity.check_order"));
    let c = d.add_node(ActivityKind::Decision, Pos::new(420.0, 160.0), "");
    let e = d.add_node(ActivityKind::Action, Pos::new(610.0, 70.0), tr("activity.approve"));
    let f = d.add_node(ActivityKind::Action, Pos::new(610.0, 250.0), tr("activity.reject"));
    let g = d.add_node(ActivityKind::Merge, Pos::new(800.0, 160.0), "");
    let h = d.add_node(ActivityKind::Final, Pos::new(930.0, 160.0), "");
    for (from, to, label) in [
        (a, b, ""),
        (b, c, ""),
        (c, e, "[valid]"),
        (c, f, "[else]"),
        (e, g, ""),
        (f, g, ""),
        (g, h, ""),
    ] {
        d.connect(from, to, label);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activity_endpoints_and_roundtrip() {
        let d = example();
        assert!(d.issues().is_empty());
        assert!(d.check_storage().is_ok());
        assert_eq!(
            serde_json::from_str::<ActivityDiagram>(&serde_json::to_string(&d).unwrap()).unwrap(),
            d
        );
        assert!(!d.can_connect(d.nodes.last().unwrap().id, d.nodes[1].id));
        assert!(!d.can_connect(d.nodes[1].id, d.nodes[0].id));
    }
    #[test]
    fn deletion_and_duplicate_preserve_graph_integrity() {
        let mut d = example();
        let ids = d.nodes.iter().map(|n| n.id).collect();
        let copied = d.duplicate(&ids);
        assert_eq!(copied.len(), 7);
        assert_eq!(d.flows.len(), 14);
        d.delete(&copied, None);
        assert_eq!(d, example());
        assert!(d.check_storage().is_ok());
        d.nodes[0].bounds.x = f32::NAN;
        assert!(d.check_storage().is_err());
    }
}
