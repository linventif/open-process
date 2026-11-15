//! Typed, portable Mermaid projections. Native documents retain notation semantics.
use super::Diagram;
use super::conversion::{Conversion, encode};
use crate::i18n::tr;
use crate::{bpmn, flow, petri, uml};
use std::collections::HashMap;
use std::fmt::Write;

fn projection(name: &str, source: String) -> Conversion<Diagram> {
    Conversion {
        diagram: Diagram { name: name.into(), source },
        warnings: vec![tr("mermaid.export_projection").into()],
    }
}

fn node(text: &mut String, id: impl std::fmt::Display, label: &str, open: &str, close: &str) {
    writeln!(text, "    {id}{open}\"{}\"{close}", encode(label)).unwrap();
}

fn edge(text: &mut String, from: impl std::fmt::Display, to: impl std::fmt::Display, arrow: &str, label: &str) {
    write!(text, "    {from} {arrow}").unwrap();
    if !label.is_empty() {
        write!(text, "|\"{}\"|", encode(label)).unwrap();
    }
    writeln!(text, " {to}").unwrap();
}

pub fn bpmn(source: &bpmn::BpmnDiagram) -> Result<Conversion<Diagram>, String> {
    source.check_storage()?;
    let mut text = String::from("flowchart LR\n");
    // Resolve geometric containment once per node, without repeated node-ID searches.
    let mut groups: HashMap<(Option<bpmn::Id>, Option<bpmn::Id>), Vec<&bpmn::Node>> = HashMap::new();
    for n in &source.nodes {
        let pool = source.pools.iter().find(|p| p.rect().contains(n.pos));
        let lane = pool.and_then(|p| p.lane_at(n.pos).map(|i| p.lanes[i].id));
        groups.entry((pool.map(|p| p.id), lane)).or_default().push(n);
    }
    let emit = |text: &mut String, nodes: &[&bpmn::Node]| {
        for n in nodes {
            use bpmn::NodeKind::*;
            let (open, close) = match n.kind {
                StartEvent(_) | IntermediateEvent { .. } => ("((", "))"),
                EndEvent(_) => ("(((", ")))"),
                Gateway(_) => ("{", "}"),
                SubProcess => ("[[", "]]"),
                DataObject => ("[/", "/]"),
                DataStore => ("[(", ")]"),
                Task(_) | TextAnnotation => ("[", "]"),
            };
            let label = if n.name.is_empty() {
                n.kind.label()
            } else {
                format!("{}\n{}", n.name, n.kind.label())
            };
            node(text, format_args!("n{}", n.id), &label, open, close);
        }
    };
    emit(&mut text, groups.get(&(None, None)).map(Vec::as_slice).unwrap_or_default());
    for pool in &source.pools {
        writeln!(text, "    subgraph pool{}[\"{}\"]", pool.id, encode(&pool.name)).unwrap();
        emit(&mut text, groups.get(&(Some(pool.id), None)).map(Vec::as_slice).unwrap_or_default());
        for lane in &pool.lanes {
            writeln!(text, "    subgraph lane{}[\"{}\"]", lane.id, encode(&lane.name)).unwrap();
            emit(
                &mut text,
                groups.get(&(Some(pool.id), Some(lane.id))).map(Vec::as_slice).unwrap_or_default(),
            );
            text.push_str("    end\n");
        }
        text.push_str("    end\n");
    }
    for f in &source.flows {
        let arrow = match f.kind {
            bpmn::FlowKind::Sequence => "-->",
            bpmn::FlowKind::Message => "-.->",
            bpmn::FlowKind::Association => "-.-",
        };
        let label = if f.is_default {
            format!("{}: {}", tr("bpmn.default_flow"), f.name)
        } else {
            f.name.clone()
        };
        edge(&mut text, format_args!("n{}", f.from), format_args!("n{}", f.to), arrow, &label);
    }
    Ok(projection(&source.name, text))
}

pub fn petri(source: &petri::PetriNet) -> Result<Conversion<Diagram>, String> {
    source.check_storage()?;
    let mut text = String::from("flowchart LR\n");
    for p in &source.places {
        let mut label = format!("{}\n{}: {}", p.name, tr("petri.tokens_m0"), p.tokens);
        if let Some(capacity) = p.capacity {
            write!(label, "\n{}: {capacity}", tr("petri.capacity")).unwrap();
        }
        node(&mut text, format_args!("p{}", p.id), &label, "((", "))");
    }
    for t in &source.transitions {
        let label = if t.silent { format!("τ {}", t.name) } else { t.name.clone() };
        node(&mut text, format_args!("t{}", t.id), &label, "[", "]");
    }
    let id = |n| match n {
        petri::NodeRef::Place(id) => format!("p{id}"),
        petri::NodeRef::Transition(id) => format!("t{id}"),
    };
    for a in &source.arcs {
        let (arrow, label) = if a.kind == petri::ArcKind::Inhibitor {
            ("--o", format!("{} < {}", tr("petri.inhibitor"), a.weight))
        } else {
            ("-->", if a.weight == 1 { String::new() } else { a.weight.to_string() })
        };
        edge(&mut text, id(a.from), id(a.to), arrow, &label);
    }
    Ok(projection(&source.name, text))
}

pub fn flow<K: flow::FlowKind>(source: &flow::Diagram<K>) -> Result<Conversion<Diagram>, String> {
    source.check_storage()?;
    let mut text = String::from("flowchart LR\n");
    for n in &source.nodes {
        let (open, close) = match n.kind.shape() {
            flow::Shape::Event => ("{{", "}}"),
            flow::Shape::Diamond => ("{", "}"),
            flow::Shape::Initial | flow::Shape::Connector(_) => ("((", "))"),
            flow::Shape::Final | flow::Shape::FlowFinal => ("(((", ")))"),
            flow::Shape::Action | flow::Shape::Bar => ("[", "]"),
        };
        let label = if n.name.is_empty() {
            tr(n.kind.label()).into()
        } else {
            format!("{}\n{}", n.name, tr(n.kind.label()))
        };
        node(&mut text, format_args!("n{}", n.id), &label, open, close);
    }
    for f in &source.flows {
        edge(&mut text, format_args!("n{}", f.from), format_args!("n{}", f.to), "-->", &f.label);
    }
    Ok(projection(&source.name, text))
}

pub fn uml(source: &uml::UmlDiagram) -> Result<Conversion<Diagram>, String> {
    if source.kind == uml::DiagramKind::Class {
        return super::conversion::from_uml(source);
    }
    source.check_storage()?;
    let mut text = String::from("flowchart LR\n");
    let mut groups: HashMap<Option<uml::Id>, Vec<&uml::Node>> = HashMap::new();
    for n in source.nodes.iter().filter(|n| n.kind != uml::NodeKind::System) {
        groups.entry(source.system_of(n)).or_default().push(n);
    }
    let emit = |text: &mut String, nodes: &[&uml::Node]| {
        for n in nodes {
            let (label, open, close) = if n.kind == uml::NodeKind::Actor {
                (format!("{}\n{}", n.name, tr(n.kind.label())), "[", "]")
            } else {
                (n.name.clone(), "([", "])")
            };
            node(text, format_args!("n{}", n.id), &label, open, close);
        }
    };
    emit(&mut text, groups.get(&None).map(Vec::as_slice).unwrap_or_default());
    for system in source.nodes.iter().filter(|n| n.kind == uml::NodeKind::System) {
        writeln!(text, "    subgraph system{}[\"{}\"]", system.id, encode(&system.name)).unwrap();
        emit(&mut text, groups.get(&Some(system.id)).map(Vec::as_slice).unwrap_or_default());
        text.push_str("    end\n");
    }
    for r in &source.relations {
        let arrow = match r.kind {
            uml::RelationKind::Association => "---",
            uml::RelationKind::Include | uml::RelationKind::Extend => "-.->",
            _ => "-->",
        };
        let label = match r.kind {
            uml::RelationKind::Association => r.label.clone(),
            uml::RelationKind::Include => format!("«include» {}", r.label),
            uml::RelationKind::Extend => format!("«extend» {}", r.label),
            kind => format!("{} {}", tr(kind.label()), r.label),
        };
        edge(&mut text, format_args!("n{}", r.from), format_args!("n{}", r.to), arrow, &label);
    }
    Ok(projection(&source.name, text))
}

/// Keep loss diagnostics in raw files as Mermaid comments.
pub fn source(diagram: &Diagram, warnings: &[String]) -> String {
    let mut text = String::new();
    for warning in warnings {
        writeln!(text, "%% {}", warning.replace(['\r', '\n'], " ")).unwrap();
    }
    text.push_str(&diagram.source);
    text
}

#[cfg(test)]
mod tests;
