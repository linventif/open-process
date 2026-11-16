//! Typed native flowcharts and conservative Mermaid interchange.
use crate::flow::{Diagram, FlowKind, Shape};
use crate::geom::Pos;
use crate::i18n::{key, tr};
use crate::mermaid::conversion::Conversion;
use mermaid_svg::ast::{EdgeHead, EdgeLine, NodeShape};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowchartKind {
    Process,
    Decision,
    Terminator,
    Connector,
    Preparation,
}
pub type FlowchartDiagram = Diagram<FlowchartKind>;

impl FlowKind for FlowchartKind {
    fn all() -> &'static [Self] {
        &[Self::Process, Self::Decision, Self::Terminator, Self::Connector, Self::Preparation]
    }
    fn label(self) -> &'static str {
        match self {
            Self::Process => key("mermaid.builder.node"),
            Self::Decision => key("mermaid.builder.decision"),
            Self::Terminator => key("flowchart.terminator"),
            Self::Connector => key("flowchart.connector"),
            Self::Preparation => key("flowchart.preparation"),
        }
    }
    fn shape(self) -> Shape {
        match self {
            Self::Process => Shape::Action,
            Self::Terminator => Shape::Terminator,
            Self::Decision => Shape::Diamond,
            Self::Connector => Shape::Connector(""),
            Self::Preparation => Shape::Event,
        }
    }
    fn can_connect(self, _: Self) -> bool {
        true
    }
    fn degree_valid(self, _: usize, _: usize) -> bool {
        true
    }
}

fn encode(text: &str) -> String {
    crate::mermaid::conversion::encode(text)
}

pub fn to_mermaid(diagram: &FlowchartDiagram) -> Result<Conversion<crate::mermaid::Diagram>, String> {
    diagram.check_storage()?;
    let mut source = String::from("flowchart LR\n");
    for node in &diagram.nodes {
        let (open, close) = match node.kind {
            FlowchartKind::Process => ("[", "]"),
            FlowchartKind::Decision => ("{", "}"),
            FlowchartKind::Terminator => ("([", "])"),
            FlowchartKind::Connector => ("((", "))"),
            FlowchartKind::Preparation => ("{{", "}}"),
        };
        writeln!(source, "    n{}{open}\"{}\"{close}", node.id, encode(&node.name)).unwrap();
    }
    for flow in &diagram.flows {
        writeln!(source, "    n{} -->|\"{}\"| n{}", flow.from, encode(&flow.label), flow.to).unwrap();
    }
    Ok(Conversion {
        diagram: crate::mermaid::Diagram {
            name: diagram.name.clone(),
            source,
        },
        warnings: vec![tr("conversion.mermaid_layout").into()],
    })
}

pub fn from_mermaid(source: &crate::mermaid::Diagram) -> Result<Conversion<FlowchartDiagram>, String> {
    let (parsed, meta) = mermaid_svg::parse_with_meta(&source.source).map_err(|e| e.to_string())?;
    let mermaid_svg::Diagram::Flowchart(graph) = parsed else {
        return Err(tr("flowchart.unsupported").into());
    };
    if !graph.subgraphs.is_empty() {
        return Err(tr("flowchart.unsupported").into());
    }
    let mut diagram = FlowchartDiagram::new(&source.name);
    let mut ids = HashMap::new();
    let mut warnings = vec![tr("conversion.mermaid_layout").into()];
    let presentation = !graph.class_defs.is_empty()
        || !graph.edge_styles.is_empty()
        || !graph.link_style_default.is_empty()
        || !graph.edge_interpolate.is_empty()
        || graph.default_interpolate.is_some()
        || graph.config_curve.is_some()
        || meta != mermaid_svg::DiagramMeta::default()
        || graph
            .nodes
            .iter()
            .any(|node| node.shape == NodeShape::Round || !node.style.is_empty() || !node.classes.is_empty())
        || graph.edges.iter().any(|edge| edge.curve.is_some() || edge.id.is_some());
    if presentation {
        warnings.push(tr("flowchart.presentation").into());
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        let kind = match node.shape {
            NodeShape::Rect => FlowchartKind::Process,
            NodeShape::Round | NodeShape::Stadium => FlowchartKind::Terminator,
            NodeShape::Rhombus => FlowchartKind::Decision,
            NodeShape::Circle => FlowchartKind::Connector,
            NodeShape::Hexagon => FlowchartKind::Preparation,
            _ => return Err(tr("flowchart.unsupported").into()),
        };
        if node.click.is_some() {
            return Err(tr("flowchart.unsupported").into());
        }
        let id = diagram.add_node(
            kind,
            Pos::new(120.0 + (index % 3) as f32 * 230.0, 100.0 + (index / 3) as f32 * 150.0),
            crate::mermaid::conversion::decode(&node.text),
        );
        ids.insert(node.id.as_str(), id);
    }
    for edge in graph.edges {
        if edge.line != EdgeLine::Solid || edge.head != EdgeHead::Arrow || edge.tail != EdgeHead::None || edge.animate {
            return Err(tr("flowchart.unsupported").into());
        }
        let from = *ids.get(edge.from.as_str()).ok_or_else(|| tr("flowchart.unsupported").to_owned())?;
        let to = *ids.get(edge.to.as_str()).ok_or_else(|| tr("flowchart.unsupported").to_owned())?;
        diagram
            .connect(from, to, crate::mermaid::conversion::decode(edge.label.as_deref().unwrap_or("")))
            .ok_or_else(|| tr("flowchart.unsupported").to_owned())?;
    }
    Ok(Conversion { diagram, warnings })
}

pub fn example() -> FlowchartDiagram {
    from_mermaid(&crate::mermaid::Diagram::new(tr("flowchart.example")))
        .unwrap()
        .diagram
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_flowcharts_roundtrip_semantics_and_reject_unsupported_source_atomically() {
        let mut diagram = FlowchartDiagram::new("Example");
        for (i, kind) in FlowchartKind::all().iter().enumerate() {
            diagram.add_node(*kind, Pos::new(i as f32 * 180.0, 100.0), format!("Label {i} <&>"));
        }
        for i in 1..diagram.nodes.len() {
            diagram
                .connect(diagram.nodes[i - 1].id, diagram.nodes[i].id, "Yes \" & next")
                .unwrap();
        }
        let source = to_mermaid(&diagram).unwrap();
        crate::mermaid::render_svg(&source.diagram.source).unwrap();
        let restored = from_mermaid(&source.diagram).unwrap().diagram;
        assert_eq!(
            diagram.nodes.iter().map(|n| (n.kind, &n.name)).collect::<Vec<_>>(),
            restored.nodes.iter().map(|n| (n.kind, &n.name)).collect::<Vec<_>>()
        );
        assert_eq!(diagram.flows, restored.flows);
        for source in [
            "flowchart LR\n A -.-> B",
            "flowchart LR\n A --> A",
            "flowchart LR\n subgraph S\n A\n end",
            "sequenceDiagram\n A->>B: Hello",
        ] {
            let draft = crate::mermaid::Diagram {
                name: "Draft".into(),
                source: source.into(),
            };
            let original = draft.clone();
            assert!(from_mermaid(&draft).is_err());
            assert_eq!(draft, original);
        }
    }
}
