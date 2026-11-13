//! EPC events, functions and logical connectors; no simulation semantics implied.
use crate::flow::{Diagram, FlowKind, Shape};
use crate::geom::Pos;
use crate::i18n::{key, tr};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpcKind {
    Event,
    Function,
    And,
    Or,
    Xor,
}
pub type EpcDiagram = Diagram<EpcKind>;
impl FlowKind for EpcKind {
    fn all() -> &'static [Self] {
        &[Self::Event, Self::Function, Self::And, Self::Or, Self::Xor]
    }
    fn label(self) -> &'static str {
        match self {
            Self::Event => key("Event"),
            Self::Function => key("Function"),
            Self::And => "AND",
            Self::Or => "OR",
            Self::Xor => "XOR",
        }
    }
    fn shape(self) -> Shape {
        match self {
            Self::Event => Shape::Event,
            Self::Function => Shape::Action,
            Self::And => Shape::Connector("∧"),
            Self::Or => Shape::Connector("∨"),
            Self::Xor => Shape::Connector("X"),
        }
    }
    fn can_connect(self, to: Self) -> bool {
        !matches!((self, to), (Self::Event, Self::Event) | (Self::Function, Self::Function))
    }
    fn degree_valid(self, i: usize, o: usize) -> bool {
        match self {
            Self::Event => (i == 0 && o == 1) || (i == 1 && o == 0) || (i == 1 && o == 1),
            Self::Function => i == 1 && o == 1,
            _ => (i == 1 && o >= 2) || (i >= 2 && o == 1),
        }
    }
}
pub fn example() -> EpcDiagram {
    let mut d = EpcDiagram::new(tr("Order processing"));
    let a = d.add_node(EpcKind::Event, Pos::new(110.0, 130.0), tr("Order received"));
    let b = d.add_node(EpcKind::Function, Pos::new(330.0, 130.0), tr("Check order"));
    let c = d.add_node(EpcKind::Event, Pos::new(550.0, 130.0), tr("Order checked"));
    let e = d.add_node(EpcKind::Function, Pos::new(770.0, 130.0), tr("Ship order"));
    let f = d.add_node(EpcKind::Event, Pos::new(990.0, 130.0), tr("Order shipped"));
    for (from, to) in [(a, b), (b, c), (c, e), (e, f)] {
        d.connect(from, to, "");
    }
    d
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn epc_rejects_invalid_direct_connections_and_dangling_flows() {
        let mut d = example();
        assert!(d.issues().is_empty());
        assert!(d.check_storage().is_ok());
        assert!(!d.can_connect(d.nodes[0].id, d.nodes[2].id));
        assert!(!d.can_connect(d.nodes[1].id, d.nodes[3].id));
        assert!(!d.can_connect(999, d.nodes[0].id));
        d.flows[0].to = 999;
        assert!(d.check_storage().is_err());
    }
    #[test]
    fn exports_escape_names_and_keep_symbols() {
        let mut d = example();
        d.nodes[0].name = "<script>&\"".into();
        let root = crate::xml::parse(&d.to_svg()).unwrap();
        assert_eq!(root.name, "svg");
        assert!(d.to_svg().contains("&lt;script&gt;&amp;&quot;"));
        assert!(d.to_dot().contains("digraph"));
    }
}
