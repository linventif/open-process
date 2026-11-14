//! Explicit UML class ↔ Mermaid conversions. Sources are never modified.
use super::Diagram;
use crate::geom::Pos;
use crate::i18n::{key, tr, trf};
use crate::uml::{DiagramKind, NodeKind, RelationKind, UmlDiagram, valid_multiplicity};
use mermaid_svg::ast::{ClassRelationKind, FlowDirection, MemberKind, Visibility};
use std::collections::HashMap;
use std::fmt::Write;

pub struct Conversion<T> {
    pub diagram: T,
    pub warnings: Vec<String>,
}

/// Emit portable classDiagram syntax, using stable aliases for arbitrary/duplicate names.
pub fn from_uml(source: &UmlDiagram) -> Result<Conversion<Diagram>, String> {
    if source.kind != DiagramKind::Class {
        return Err(tr("conversion.mermaid_classes_only").into());
    }
    let issues = source.validate();
    if !issues.is_empty() {
        return Err(issues.into_iter().map(|(_, message)| message).collect::<Vec<_>>().join(" · "));
    }
    let mut text = String::from("classDiagram\n");
    let mut warnings = Vec::new();
    if !source.nodes.is_empty() {
        warn(&mut warnings, key("conversion.mermaid_layout"));
    }
    for node in &source.nodes {
        writeln!(text, "    class n{}[\"{}\"] {{", node.id, encode(&node.name)).unwrap();
        let stereotype = match node.kind {
            NodeKind::Interface => Some("interface"),
            NodeKind::Enumeration => Some("enumeration"),
            NodeKind::Class if node.abstract_class => Some("abstract"),
            _ => None,
        };
        if let Some(stereotype) = stereotype {
            writeln!(text, "        <<{stereotype}>>").unwrap();
        }
        for (members, methods) in [(&node.attributes, false), (&node.operations, true)] {
            for line in members.lines().map(str::trim).filter(|line| !line.is_empty()) {
                // Mermaid separates methods from attributes by the presence of parentheses.
                // Reject ambiguous member compartments instead of silently moving their contents.
                if line.contains('(') != methods {
                    return Err(trf("conversion.mermaid_member", &[&node.name, &line]));
                }
                let (visibility, body) = split_visibility(line);
                writeln!(text, "        {visibility}{}", encode(body)).unwrap();
            }
        }
        writeln!(text, "    }}").unwrap();
    }
    for relation in &source.relations {
        let arrow = match relation.kind {
            RelationKind::Association => "--",
            RelationKind::DirectedAssociation => "-->",
            RelationKind::Aggregation => "o--",
            RelationKind::Composition => "*--",
            RelationKind::Generalization => "--|>",
            RelationKind::Realization => "..|>",
            RelationKind::Dependency => "..>",
            _ => return Err(tr("conversion.mermaid_classes_only").into()),
        };
        if !relation.source_role.is_empty() || !relation.target_role.is_empty() {
            warn(&mut warnings, key("conversion.mermaid_roles"));
        }
        write!(text, "    n{}", relation.from).unwrap();
        if !relation.source_multiplicity.is_empty() {
            write!(text, " \"{}\"", encode(&relation.source_multiplicity)).unwrap();
        }
        write!(text, " {arrow}").unwrap();
        if !relation.target_multiplicity.is_empty() {
            write!(text, " \"{}\"", encode(&relation.target_multiplicity)).unwrap();
        }
        write!(text, " n{}", relation.to).unwrap();
        if !relation.label.is_empty() {
            write!(text, " : {}", encode(&relation.label)).unwrap();
        }
        text.push('\n');
    }
    mermaid_svg::parse(&text).map_err(|e| e.to_string())?;
    Ok(Conversion {
        diagram: Diagram {
            name: source.name.clone(),
            source: text,
        },
        warnings,
    })
}

/// Convert the typed Mermaid AST into the builder's validated model.
pub fn to_uml(source: &Diagram) -> Result<Conversion<UmlDiagram>, String> {
    let (parsed, meta) = mermaid_svg::parse_with_meta(&source.source).map_err(|e| e.to_string())?;
    let mermaid_svg::Diagram::Class(classes) = parsed else {
        return Err(tr("conversion.mermaid_classes_only").into());
    };
    check_blocks(&source.source)?;
    let mut diagram = UmlDiagram::new(&source.name, DiagramKind::Class);
    let mut warnings = Vec::new();
    let mut ids = HashMap::new();
    if !classes.classes.is_empty() {
        warn(&mut warnings, key("conversion.mermaid_layout"));
    }
    if !classes.namespaces.is_empty() || !classes.notes.is_empty() {
        warn(&mut warnings, key("conversion.mermaid_annotations"));
    }
    if !classes.class_defs.is_empty()
        || meta.theme.is_some()
        || !meta.config.is_empty()
        || meta.font_family.is_some()
        || meta.font_size.is_some()
        || !meta.theme_variables.is_empty()
    {
        warn(&mut warnings, key("conversion.mermaid_styles"));
    }
    if meta.acc_title.is_some() || meta.acc_descr.is_some() || meta.title.is_some() {
        warn(&mut warnings, key("conversion.mermaid_annotations"));
    }
    // Mermaid allows realization targets without an explicit interface declaration.
    // Infer only unannotated targets; explicit incompatible stereotypes remain errors.
    let interfaces: Vec<_> = classes
        .relations
        .iter()
        .filter(|r| r.kind == ClassRelationKind::Realization && r.to_kind.is_none())
        .map(|r| if r.reversed { r.from.as_str() } else { r.to.as_str() })
        .collect();
    for class in &classes.classes {
        let stereotype = class.stereotype.as_deref().unwrap_or("").to_ascii_lowercase();
        let kind = match stereotype.as_str() {
            "interface" => NodeKind::Interface,
            "enumeration" | "enum" => NodeKind::Enumeration,
            "" if interfaces.contains(&class.name.as_str()) => {
                warn(&mut warnings, key("conversion.mermaid_inferred_interface"));
                NodeKind::Interface
            }
            "" | "abstract" => NodeKind::Class,
            _ => {
                warn(&mut warnings, key("conversion.mermaid_stereotypes"));
                NodeKind::Class
            }
        };
        if !class.style.is_empty() || !class.classes.is_empty() {
            warn(&mut warnings, key("conversion.mermaid_styles"));
        }
        if class.click.is_some() {
            warn(&mut warnings, key("conversion.mermaid_interactions"));
        }
        let name = decode(class.label.as_deref().unwrap_or(&class.name));
        let id = diagram.add_node(kind, Pos::default(), name).unwrap();
        ids.insert(class.name.as_str(), id);
        let node = diagram.nodes.last_mut().unwrap();
        node.abstract_class = stereotype == "abstract";
        let mut attributes = Vec::new();
        let mut operations = Vec::new();
        for member in &class.members {
            let visibility = match member.visibility {
                Visibility::Default => "",
                Visibility::Public => "+",
                Visibility::Private => "-",
                Visibility::Protected => "#",
                Visibility::Package => "~",
                _ => return Err(tr("conversion.mermaid_unsupported_member").into()),
            };
            let text = decode(&member.text);
            let line = if visibility.is_empty() {
                text
            } else {
                format!("{visibility} {text}")
            };
            match member.kind {
                MemberKind::Attribute => attributes.push(line),
                MemberKind::Method => operations.push(line),
                _ => return Err(tr("conversion.mermaid_unsupported_member").into()),
            }
        }
        node.attributes = attributes.join("\n");
        node.operations = operations.join("\n");
    }
    for relation in &classes.relations {
        let invalid = || trf("conversion.mermaid_relation", &[&relation.from, &relation.to]);
        if relation.to_kind.is_some() || relation.lollipop_from || relation.lollipop_to {
            return Err(invalid());
        }
        let (kind, swap) = match relation.kind {
            ClassRelationKind::Link => (RelationKind::Association, false),
            ClassRelationKind::Association => (RelationKind::DirectedAssociation, relation.reversed),
            ClassRelationKind::Composition => (RelationKind::Composition, !relation.reversed),
            ClassRelationKind::Aggregation => (RelationKind::Aggregation, !relation.reversed),
            ClassRelationKind::Inheritance => (RelationKind::Generalization, relation.reversed),
            ClassRelationKind::Realization => (RelationKind::Realization, relation.reversed),
            ClassRelationKind::Dependency => (RelationKind::Dependency, relation.reversed),
            _ => return Err(invalid()),
        };
        let mut from = *ids.get(relation.from.as_str()).ok_or_else(invalid)?;
        let mut to = *ids.get(relation.to.as_str()).ok_or_else(invalid)?;
        let mut from_card = decode(relation.from_card.as_deref().unwrap_or(""));
        let mut to_card = decode(relation.to_card.as_deref().unwrap_or(""));
        if swap {
            std::mem::swap(&mut from, &mut to);
            std::mem::swap(&mut from_card, &mut to_card);
        }
        // Arbitrary Mermaid endpoint text is an end role when it isn't a UML multiplicity.
        let from_role = if valid_multiplicity(&from_card) {
            String::new()
        } else {
            std::mem::take(&mut from_card)
        };
        let to_role = if valid_multiplicity(&to_card) {
            String::new()
        } else {
            std::mem::take(&mut to_card)
        };
        if !from_role.is_empty() || !to_role.is_empty() {
            warn(&mut warnings, key("conversion.mermaid_endpoint_roles"));
        }
        diagram.connect(from, to, kind).ok_or_else(invalid)?;
        let target = diagram.relations.last_mut().unwrap();
        target.label = decode(relation.label.as_deref().unwrap_or(""));
        target.source_multiplicity = from_card;
        target.target_multiplicity = to_card;
        target.source_role = from_role;
        target.target_role = to_role;
    }
    arrange(&mut diagram, classes.direction);
    let issues = diagram.validate();
    if !issues.is_empty() {
        return Err(issues.into_iter().map(|(_, message)| message).collect::<Vec<_>>().join(" · "));
    }
    Ok(Conversion { diagram, warnings })
}

fn warn(warnings: &mut Vec<String>, key: &'static str) {
    let message = tr(key).to_string();
    if !warnings.contains(&message) {
        warnings.push(message);
    }
}

fn split_visibility(line: &str) -> (&str, &str) {
    if line.starts_with(['+', '-', '#', '~']) {
        (&line[..1], line[1..].trim())
    } else {
        ("", line)
    }
}

/// Mermaid entity codes keep user text from becoming declarations or HTML markup.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if matches!(
            c,
            '#' | '%' | '"' | '\'' | '{' | '}' | '[' | ']' | '<' | '>' | '&' | '`' | '\n' | '\r' | ';'
        ) {
            write!(out, "#{};", c as u32).unwrap();
        } else {
            out.push(c);
        }
    }
    out
}

fn decode(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(body) = rest.strip_prefix('#')
            && let Some((token, _)) = body.split_once(';')
        {
            let value = if let Some(hex) = token.strip_prefix(['x', 'X']) {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
            } else {
                token.parse::<u32>().ok().and_then(char::from_u32)
            };
            let value = value.or(match token {
                "quot" => Some('"'),
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                "colon" => Some(':'),
                "semi" => Some(';'),
                "hearts" => Some('♥'),
                _ => None,
            });
            if let Some(value) = value {
                out.push(value);
                rest = &rest[token.len() + 2..];
                continue;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// The dependency accepts EOF in an open class/namespace body; conversion must reject incomplete drafts.
fn check_blocks(source: &str) -> Result<(), String> {
    let mut started = false;
    let mut depth = 0usize;
    for line in source.lines().map(str::trim) {
        if matches!(
            line.trim_start_matches('\u{feff}').split_whitespace().next(),
            Some("classDiagram" | "classDiagram-v2")
        ) {
            started = true;
            continue;
        }
        if !started || line.starts_with("%%") {
            continue;
        }
        let mut quote = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                quote = !quote;
            }
            if quote {
                continue;
            }
            if c == '%' && chars.peek() == Some(&'%') {
                break;
            }
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| tr("conversion.mermaid_incomplete").to_string())?;
            }
        }
        if quote {
            return Err(tr("conversion.mermaid_incomplete").into());
        }
    }
    if depth != 0 {
        return Err(tr("conversion.mermaid_incomplete").into());
    }
    Ok(())
}

fn arrange(diagram: &mut UmlDiagram, direction: FlowDirection) {
    let mut widths = [240.0f32; 3];
    for (index, node) in diagram.nodes.iter_mut().enumerate() {
        let longest = std::iter::once(node.name.as_str())
            .chain(node.attributes.lines())
            .chain(node.operations.lines())
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        node.bounds.w = (longest as f32 * 8.0 + 40.0).clamp(240.0, 520.0);
        widths[index % 3] = widths[index % 3].max(node.bounds.w);
    }
    let mut y = 40.0;
    for row in diagram.nodes.chunks_mut(3) {
        let mut x = 40.0;
        let height = row.iter().map(|n| n.rect().h).fold(0.0, f32::max);
        for (column, node) in row.iter_mut().enumerate() {
            node.bounds.x = x;
            node.bounds.y = y;
            x += widths[column] + 100.0;
        }
        y += height + 100.0;
    }
    if matches!(direction, FlowDirection::RightLeft | FlowDirection::BottomTop) {
        let right = diagram.nodes.iter().map(|n| n.rect().right()).fold(0.0, f32::max);
        let bottom = diagram.nodes.iter().map(|n| n.rect().bottom()).fold(0.0, f32::max);
        for node in &mut diagram.nodes {
            if direction == FlowDirection::RightLeft {
                node.bounds.x = right - node.bounds.x - node.rect().w + 40.0;
            } else {
                node.bounds.y = bottom - node.bounds.y - node.rect().h + 40.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(source: &str) -> Diagram {
        Diagram {
            name: "Classes".into(),
            source: source.into(),
        }
    }

    fn assert_semantics(before: &UmlDiagram, after: &UmlDiagram) {
        assert_eq!(before.name, after.name);
        assert_eq!(before.nodes.len(), after.nodes.len());
        assert_eq!(before.relations.len(), after.relations.len());
        for (a, b) in before.nodes.iter().zip(&after.nodes) {
            assert_eq!(
                (&a.kind, &a.name, &a.attributes, &a.operations, a.abstract_class),
                (&b.kind, &b.name, &b.attributes, &b.operations, b.abstract_class)
            );
        }
        for (a, b) in before.relations.iter().zip(&after.relations) {
            let index = |d: &UmlDiagram, id| d.nodes.iter().position(|n| n.id == id).unwrap();
            assert_eq!(
                (index(before, a.from), index(before, a.to)),
                (index(after, b.from), index(after, b.to))
            );
            assert_eq!(
                (&a.kind, &a.label, &a.source_multiplicity, &a.target_multiplicity),
                (&b.kind, &b.label, &b.source_multiplicity, &b.target_multiplicity)
            );
        }
        assert!(after.validate().is_empty());
    }

    #[test]
    fn builder_example_roundtrips_and_sources_remain_unchanged() {
        let source = crate::examples::uml_classes();
        let before = source.clone();
        let converted = from_uml(&source).unwrap();
        let source_text = converted.diagram.clone();
        assert!(super::super::render_svg(&converted.diagram.source).is_ok());
        let restored = to_uml(&converted.diagram).unwrap();
        assert_semantics(&source, &restored.diagram);
        assert_eq!(source, before);
        assert_eq!(converted.diagram, source_text);
        assert_eq!(converted.warnings, [tr("conversion.mermaid_layout")]);
        assert!(
            restored
                .diagram
                .nodes
                .iter()
                .all(|n| n.bounds.x.is_finite() && n.bounds.y.is_finite())
        );
    }

    #[test]
    fn classifier_kinds_all_seven_relations_and_special_text_roundtrip() {
        let mut source = UmlDiagram::new("Escaping", DiagramKind::Class);
        let a = source
            .add_node(NodeKind::Class, Pos::default(), "École \"A\" [T] #35;\nnext")
            .unwrap();
        let b = source
            .add_node(NodeKind::Class, Pos::default(), "École \"A\" [T] #35;\nnext")
            .unwrap();
        let interface = source.add_node(NodeKind::Interface, Pos::default(), "I<Payable>").unwrap();
        source.add_node(NodeKind::Enumeration, Pos::default(), "State").unwrap();
        source.nodes[0].abstract_class = true;
        source.nodes[0].attributes = "# name: List<T> %% {field}\n~ path: String\nplain: #value".into();
        source.nodes[0].operations = "+ call(\"x\"): T\n- doThing(): bool".into();
        source.nodes[3].attributes = "READY\nDONE".into();
        for kind in DiagramKind::Class.relation_kinds() {
            let to = if *kind == RelationKind::Realization { interface } else { b };
            source.connect(a, to, *kind).unwrap();
            let r = source.relations.last_mut().unwrap();
            r.label = "uses: \"<data>\" %% #35;\nnext".into();
            r.source_multiplicity = "0..1".into();
            r.target_multiplicity = "1..*".into();
        }
        let converted = from_uml(&source).unwrap();
        assert!(super::super::render_svg(&converted.diagram.source).is_ok());
        assert_semantics(&source, &to_uml(&converted.diagram).unwrap().diagram);
    }

    #[test]
    fn reversed_markers_keep_semantic_direction_and_multiplicities() {
        let source = input(
            "classDiagram\nParent <|-- Child\nPart \"*\" --* \"1\" Whole\nA \"0..1\" <-- \"1..*\" B\nContract <|.. Implementation\nLeaf --o Root\nTarget <.. Caller\n",
        );
        let converted = to_uml(&source).unwrap();
        let d = converted.diagram;
        for (r, (from, to, kind)) in d.relations.iter().zip([
            ("Child", "Parent", RelationKind::Generalization),
            ("Whole", "Part", RelationKind::Composition),
            ("B", "A", RelationKind::DirectedAssociation),
            ("Implementation", "Contract", RelationKind::Realization),
            ("Root", "Leaf", RelationKind::Aggregation),
            ("Caller", "Target", RelationKind::Dependency),
        ]) {
            assert_eq!(
                (&*d.node(r.from).unwrap().name, &*d.node(r.to).unwrap().name, r.kind),
                (from, to, kind)
            );
        }
        assert_eq!(
            (&*d.relations[1].source_multiplicity, &*d.relations[1].target_multiplicity),
            ("1", "*")
        );
        assert_eq!(
            (&*d.relations[2].source_multiplicity, &*d.relations[2].target_multiplicity),
            ("1..*", "0..1")
        );
        assert_eq!(d.nodes.iter().find(|n| n.name == "Contract").unwrap().kind, NodeKind::Interface);
        assert!(
            converted
                .warnings
                .contains(&tr("conversion.mermaid_inferred_interface").to_string())
        );
    }

    #[test]
    fn unsupported_syntax_and_invariants_fail_without_partial_results() {
        for source in [
            "flowchart LR\nA-->B",
            "classDiagram\nclass A {",
            "classDiagram\nclass A[\"unfinished]",
            "classDiagram\nA --|> B\nB --|> A",
            "classDiagram\nA <--> B",
            "classDiagram\nA ()-- B",
            "classDiagram\nA .. B",
            "classDiagram\nclass A <<enumeration>>\nA --|> B",
        ] {
            assert!(to_uml(&input(source)).is_err(), "{source}");
        }
        assert!(from_uml(&crate::examples::uml_use_cases()).is_err());
        let mut source = crate::examples::uml_classes();
        source.nodes[0].operations = "missingParentheses".into();
        assert!(from_uml(&source).is_err());
        source.nodes[0].operations.clear();
        source.nodes[0].attributes = "value: call()".into();
        assert!(from_uml(&source).is_err());
    }

    #[test]
    fn losses_and_endpoint_interpretation_are_explicit() {
        let mut builder = crate::examples::uml_classes();
        builder.relations[0].source_role = "customer".into();
        let output = from_uml(&builder).unwrap();
        assert!(output.warnings.contains(&tr("conversion.mermaid_roles").to_string()));
        let source = input(
            "classDiagram\nnamespace Shop {\nclass Order <<service>>\n}\nnote for Order \"A note\"\nstyle Order fill:red\nOrder \"owner\" --> \"many\" Item\nclick Order href \"https://example.com\"\n",
        );
        let converted = to_uml(&source).unwrap();
        for key in [
            "conversion.mermaid_annotations",
            "conversion.mermaid_styles",
            "conversion.mermaid_stereotypes",
            "conversion.mermaid_interactions",
            "conversion.mermaid_endpoint_roles",
        ] {
            assert!(converted.warnings.contains(&tr(key).to_string()), "{key}");
        }
        let r = &converted.diagram.relations[0];
        assert_eq!((&*r.source_role, &*r.target_role), ("owner", "many"));
        assert!(r.source_multiplicity.is_empty() && r.target_multiplicity.is_empty());
        assert!(converted.diagram.validate().is_empty());
    }

    #[test]
    fn frontmatter_directives_comments_and_layout_are_supported() {
        let text = "---\ntitle: Classes\n---\n%%{init: {theme: 'dark'}}%%\nclassDiagram\nclass A { %% { ignored comment\n + run()\n}\nclass B\nclass C\nclass D\nA --> B\n";
        assert!(super::super::is_class_source(text));
        let a = to_uml(&input(text)).unwrap().diagram;
        let b = to_uml(&input(text)).unwrap().diagram;
        assert_eq!(a, b);
        for (index, n) in a.nodes.iter().enumerate() {
            for m in &a.nodes[index + 1..] {
                let x_overlap = n.rect().x < m.rect().right() && m.rect().x < n.rect().right();
                let y_overlap = n.rect().y < m.rect().bottom() && m.rect().y < n.rect().bottom();
                assert!(!(x_overlap && y_overlap));
            }
        }
        let empty = UmlDiagram::new("Empty", DiagramKind::Class);
        assert_semantics(&empty, &to_uml(&from_uml(&empty).unwrap().diagram).unwrap().diagram);
    }
}
