//! PNML (ISO/IEC 15909-2) import/export for Place/Transition nets.
//! Interoperable with WoPeD, PIPE, ProM, TINA, CPN-like tools, etc.

use crate::geom::Pos;
use crate::petri::{Arc, ArcKind, NodeRef, PetriNet, Place, Transition};
use crate::xml::{self, Element, XmlWriter};
use std::collections::HashMap;

pub fn export(net: &PetriNet) -> String {
    let mut w = XmlWriter::new();
    w.open("pnml", &[("xmlns", "http://www.pnml.org/version-2009/grammar/pnml".into())]);
    w.open(
        "net",
        &[
            ("id", "net1".into()),
            ("type", "http://www.pnml.org/version-2009/grammar/ptnet".into()),
        ],
    );
    w.open("name", &[]);
    w.text_element("text", &[], &net.name);
    w.close("name");
    w.open("page", &[("id", "page1".into())]);
    let pos = |w: &mut XmlWriter, p: Pos| {
        w.open("graphics", &[]);
        w.empty("position", &[("x", format!("{}", p.x.round())), ("y", format!("{}", p.y.round()))]);
        w.close("graphics");
    };
    for p in &net.places {
        w.open("place", &[("id", format!("p{}", p.id))]);
        w.open("name", &[]);
        w.text_element("text", &[], &p.name);
        w.close("name");
        pos(&mut w, p.pos);
        if p.tokens > 0 {
            w.open("initialMarking", &[]);
            w.text_element("text", &[], &p.tokens.to_string());
            w.close("initialMarking");
        }
        if let Some(c) = p.capacity {
            w.open("toolspecific", &[("tool", "open-process".into()), ("version", "1".into())]);
            w.text_element("capacity", &[], &c.to_string());
            w.close("toolspecific");
        }
        w.close("place");
    }
    for t in &net.transitions {
        w.open("transition", &[("id", format!("t{}", t.id))]);
        w.open("name", &[]);
        w.text_element("text", &[], &t.name);
        w.close("name");
        pos(&mut w, t.pos);
        if t.silent {
            w.open("toolspecific", &[("tool", "open-process".into()), ("version", "1".into())]);
            w.text_element("silent", &[], "true");
            w.close("toolspecific");
        }
        w.close("transition");
    }
    let nid = |n: NodeRef| match n {
        NodeRef::Place(id) => format!("p{id}"),
        NodeRef::Transition(id) => format!("t{id}"),
    };
    for a in &net.arcs {
        w.open(
            "arc",
            &[("id", format!("a{}", a.id)), ("source", nid(a.from)), ("target", nid(a.to))],
        );
        if a.weight != 1 {
            w.open("inscription", &[]);
            w.text_element("text", &[], &a.weight.to_string());
            w.close("inscription");
        }
        if a.kind == ArcKind::Inhibitor {
            w.empty("type", &[("value", "inhibitor".into())]);
        }
        w.close("arc");
    }
    w.close("page");
    w.close("net");
    w.close("pnml");
    w.out
}

/// Reads an integer from `<x><text>N</text></x>` or `<x><value>Default,N</value></x>`.
fn int_label(e: &Element, label: &str) -> Option<u32> {
    let l = e.child(label)?;
    let txt = l.text_at(&["text"]).or_else(|| l.text_at(&["value"]))?;
    txt.rsplit(',').next()?.trim().parse().ok()
}

fn position(e: &Element) -> Option<Pos> {
    let p = e.child("graphics")?.child("position")?;
    Some(Pos::new(p.attr_f32("x")?, p.attr_f32("y")?))
}

pub fn import(src: &str) -> Result<PetriNet, String> {
    let root = xml::parse(src)?;
    let net_el = if root.name == "net" {
        &root
    } else {
        root.find("net").ok_or("No <net> element found")?
    };
    let mut net = PetriNet::new(
        net_el
            .text_at(&["name", "text"])
            .or_else(|| net_el.text_at(&["name", "value"]))
            .filter(|s| !s.is_empty())
            .or(net_el.attr("id"))
            .unwrap_or("Imported net"),
    );
    let mut ids: HashMap<String, NodeRef> = HashMap::new();
    let mut next = 1u32;
    let mut auto = 0usize;
    let mut fallback = || {
        auto += 1;
        Pos::new(80.0 + 120.0 * ((auto - 1) % 8) as f32, 80.0 + 120.0 * ((auto - 1) / 8) as f32)
    };
    let all = net_el.descendants();
    for e in all.iter().filter(|e| e.name == "place") {
        let Some(xid) = e.attr("id") else { continue };
        let id = next;
        next += 1;
        let capacity = e
            .children_named("toolspecific")
            .find_map(|t| t.text_at(&["capacity"]).and_then(|c| c.parse().ok()))
            .or_else(|| int_label(e, "capacity").filter(|&c| c > 0));
        net.places.push(Place {
            id,
            name: e
                .text_at(&["name", "text"])
                .or_else(|| e.text_at(&["name", "value"]))
                .unwrap_or(xid)
                .to_string(),
            tokens: int_label(e, "initialMarking").unwrap_or(0),
            capacity,
            pos: position(e).unwrap_or_else(&mut fallback),
        });
        ids.insert(xid.to_string(), NodeRef::Place(id));
    }
    for e in all.iter().filter(|e| e.name == "transition") {
        let Some(xid) = e.attr("id") else { continue };
        let id = next;
        next += 1;
        let silent = e.children_named("toolspecific").any(|t| t.text_at(&["silent"]) == Some("true"));
        net.transitions.push(Transition {
            id,
            name: e
                .text_at(&["name", "text"])
                .or_else(|| e.text_at(&["name", "value"]))
                .unwrap_or(xid)
                .to_string(),
            pos: position(e).unwrap_or_else(&mut fallback),
            silent,
        });
        ids.insert(xid.to_string(), NodeRef::Transition(id));
    }
    for e in all.iter().filter(|e| e.name == "arc") {
        let (Some(s), Some(t)) = (e.attr("source"), e.attr("target")) else {
            continue;
        };
        let (Some(&from), Some(&to)) = (ids.get(s), ids.get(t)) else {
            continue;
        };
        if matches!(
            (from, to),
            (NodeRef::Place(_), NodeRef::Place(_)) | (NodeRef::Transition(_), NodeRef::Transition(_))
        ) {
            continue;
        }
        let inhibitor = e.child("type").and_then(|t| t.attr("value")) == Some("inhibitor");
        net.arcs.push(Arc {
            id: next,
            from,
            to,
            weight: int_label(e, "inscription").unwrap_or(1).max(1),
            kind: if inhibitor { ArcKind::Inhibitor } else { ArcKind::Normal },
        });
        next += 1;
    }
    Ok(net)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut n = PetriNet::new("demo");
        let p = n.add_place(Pos::new(10.0, 20.0), 3);
        let t = n.add_transition(Pos::new(100.0, 20.0));
        let q = n.add_place(Pos::new(200.0, 20.0), 0);
        n.add_arc(NodeRef::Place(p), NodeRef::Transition(t)).unwrap();
        n.add_arc(NodeRef::Place(p), NodeRef::Transition(t)).unwrap();
        n.add_arc(NodeRef::Transition(t), NodeRef::Place(q)).unwrap();
        n.place_mut(q).unwrap().capacity = Some(4);
        let x = export(&n);
        let b = import(&x).unwrap();
        assert_eq!(b.name, "demo");
        assert_eq!(b.initial_marking(), vec![3, 0]);
        assert_eq!(b.pre_matrix(), n.pre_matrix());
        assert_eq!(b.post_matrix(), n.post_matrix());
        assert_eq!(b.places[1].capacity, Some(4));
        assert_eq!(b.places[0].pos, Pos::new(10.0, 20.0));
    }

    #[test]
    fn pipe_style() {
        let src = r#"<pnml><net id="n" type="P/T net">
          <place id="P0"><name><value>P0</value></name><initialMarking><value>Default,2</value></initialMarking></place>
          <transition id="T0"><name><value>T0</value></name></transition>
          <arc id="A" source="P0" target="T0"><inscription><value>Default,1</value></inscription></arc>
        </net></pnml>"#;
        let n = import(src).unwrap();
        assert_eq!(n.initial_marking(), vec![2]);
        assert_eq!(n.arcs.len(), 1);
    }
}
