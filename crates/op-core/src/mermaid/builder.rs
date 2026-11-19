//! Form-based Mermaid authoring. Source remains authoritative, including unknown syntax.
//! Edits replace one recognized statement; switching views never serializes an AST.
use super::{Diagram, TEMPLATES, Template, conversion};
use crate::i18n::{key, tr, trf};
use std::ops::Range;

pub const MAX_BUILDER_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    Identifier,
    Quoted,
    /// One unquoted label, excluding delimiters that could introduce syntax.
    Label,
    Number {
        min: f64,
        max: f64,
    },
    Numbers,
    Unsigned,
    Choice(&'static [&'static str]),
    Date,
    Duration,
}

pub struct Field {
    pub label: &'static str,
    pub kind: FieldKind,
    pub default: &'static str,
}

pub struct Recipe {
    pub id: &'static str,
    pub family: &'static str,
    pub label: &'static str,
    pub fields: &'static [Field],
    // Slots occur once each, in field order. Literal Mermaid braces remain literal.
    pattern: &'static str,
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub range: Range<usize>,
    pub line: usize,
    pub recipe: &'static Recipe,
    pub values: Vec<String>,
    /// Exact source guard against stale edits, including edits made in another view.
    original: String,
}

impl std::fmt::Debug for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recipe").field("id", &self.id).finish()
    }
}

const fn field(label: &'static str, kind: FieldKind, default: &'static str) -> Field {
    Field { label, kind, default }
}
const fn id(default: &'static str) -> Field {
    field(key("mermaid.builder.identifier"), FieldKind::Identifier, default)
}
const fn from(default: &'static str) -> Field {
    field(key("mermaid.builder.from"), FieldKind::Identifier, default)
}
const fn to(default: &'static str) -> Field {
    field(key("mermaid.builder.to"), FieldKind::Identifier, default)
}
const fn text(default: &'static str) -> Field {
    field(key("mermaid.builder.text"), FieldKind::Quoted, default)
}
const fn name(default: &'static str) -> Field {
    field(key("common.name"), FieldKind::Quoted, default)
}
const fn description(default: &'static str) -> Field {
    field(key("mermaid.builder.description"), FieldKind::Quoted, default)
}
const fn technology(default: &'static str) -> Field {
    field(key("mermaid.builder.technology"), FieldKind::Quoted, default)
}
const fn coordinate(label: &'static str, default: &'static str) -> Field {
    field(label, FieldKind::Number { min: 0.0, max: 1.0 }, default)
}
const fn label(default: &'static str) -> Field {
    field(key("mermaid.builder.text"), FieldKind::Label, default)
}
const fn number(default: &'static str, min: f64, max: f64) -> Field {
    field(key("mermaid.builder.value"), FieldKind::Number { min, max }, default)
}
const fn numbers(default: &'static str) -> Field {
    field(key("mermaid.builder.values"), FieldKind::Numbers, default)
}
const fn choice(label: &'static str, values: &'static [&'static str], default: &'static str) -> Field {
    field(label, FieldKind::Choice(values), default)
}

macro_rules! recipe {
    ($id:literal, $family:literal, $label:expr, $pattern:literal, [$($field:expr),* $(,)?]) => {
        Recipe { id: $id, family: concat!("mermaid.", $family), label: $label, pattern: $pattern, fields: &[$($field),*] }
    };
}

// Domain syntax belongs here, independently of the shell, canvas or platform.
pub const RECIPES: &[Recipe] = &[
    recipe!(
        "flow.node",
        "flowchart",
        key("mermaid.builder.node"),
        "$0[$1]",
        [id("N1"), text("New node")]
    ),
    recipe!(
        "flow.decision",
        "flowchart",
        key("mermaid.builder.decision"),
        "$0{$1}",
        [id("D1"), text("Approved?")]
    ),
    recipe!(
        "flow.edge",
        "flowchart",
        key("mermaid.builder.relation"),
        "$0 -->|$1| $2",
        [from("A"), text("Next"), to("B")]
    ),
    recipe!(
        "sequence.participant",
        "sequence",
        key("mermaid.builder.participant"),
        "participant $0 as $1",
        [id("Actor1"), label("Customer")]
    ),
    recipe!(
        "sequence.message",
        "sequence",
        key("mermaid.builder.message"),
        "$0->>$1: $2",
        [from("User"), to("App"), label("Request")]
    ),
    recipe!(
        "sequence.reply",
        "sequence",
        key("mermaid.builder.reply"),
        "$0-->>$1: $2",
        [from("App"), to("User"), label("Response")]
    ),
    recipe!(
        "class.node",
        "class",
        key("mermaid.builder.class"),
        "class $0[$1]",
        [id("Class1"), text("New class")]
    ),
    recipe!(
        "class.attribute",
        "class",
        key("mermaid.builder.attribute"),
        "$0 : $1 $2",
        [id("Order"), label("int"), id("id")]
    ),
    recipe!(
        "class.operation",
        "class",
        key("mermaid.builder.operation"),
        "$0 : $1()",
        [id("Order"), id("confirm")]
    ),
    recipe!(
        "class.edge",
        "class",
        key("mermaid.builder.relation"),
        "$0 --> $1 : $2",
        [from("Order"), to("Item"), label("contains")]
    ),
    recipe!(
        "state.node",
        "state",
        key("mermaid.builder.state"),
        "state $0 as $1",
        [text("New state"), id("State1")]
    ),
    recipe!(
        "state.edge",
        "state",
        key("mermaid.builder.transition"),
        "$0 --> $1: $2",
        [from("Draft"), to("Review"), label("Submit")]
    ),
    recipe!(
        "state.initial",
        "state",
        key("mermaid.builder.initial"),
        "[*] --> $0",
        [to("Draft")]
    ),
    recipe!(
        "state.final",
        "state",
        key("mermaid.builder.final"),
        "$0 --> [*]",
        [from("Approved")]
    ),
    recipe!(
        "er.entity",
        "er",
        key("mermaid.builder.entity"),
        "$0 {\n        $1 $2\n    }",
        [id("ENTITY1"), id("int"), id("id")]
    ),
    recipe!(
        "er.edge",
        "er",
        key("mermaid.builder.relation"),
        "$0 ||--o{ $1 : $2",
        [from("CUSTOMER"), to("ORDER"), label("places")]
    ),
    recipe!(
        "pie.value",
        "pie",
        key("mermaid.builder.data"),
        "$0 : $1",
        [text("New category"), number("1", 0.0, f64::MAX)]
    ),
    recipe!(
        "gantt.section",
        "gantt",
        key("mermaid.builder.section"),
        "section $0",
        [label("New section")]
    ),
    recipe!(
        "gantt.task",
        "gantt",
        key("mermaid.builder.task"),
        "$0 :$1, $2, $3",
        [
            label("New task"),
            id("task1"),
            field(key("mermaid.builder.date"), FieldKind::Date, "2026-01-01"),
            field(key("mermaid.builder.duration"), FieldKind::Duration, "3d")
        ]
    ),
    recipe!(
        "gantt.after",
        "gantt",
        key("mermaid.builder.dependent_task"),
        "$0 :$1, after $2, $3",
        [
            label("Next task"),
            id("task2"),
            from("design"),
            field(key("mermaid.builder.duration"), FieldKind::Duration, "3d")
        ]
    ),
    recipe!(
        "journey.section",
        "journey",
        key("mermaid.builder.section"),
        "section $0",
        [label("New section")]
    ),
    recipe!(
        "journey.task",
        "journey",
        key("mermaid.builder.task"),
        "$0: $1: $2",
        [
            label("New task"),
            choice(key("mermaid.builder.score"), &["1", "2", "3", "4", "5"], "3"),
            label("Customer")
        ]
    ),
    recipe!(
        "timeline.event",
        "timeline",
        key("mermaid.builder.event"),
        "$0 : $1",
        [label("2028"), label("New event")]
    ),
    recipe!(
        "timeline.section",
        "timeline",
        key("mermaid.builder.section"),
        "section $0",
        [label("New section")]
    ),
    recipe!(
        "sankey.link",
        "sankey",
        key("mermaid.builder.relation"),
        "$0,$1,$2",
        [label("Source"), label("Destination"), number("1", 0.0, f64::MAX)]
    ),
    recipe!(
        "quadrant.point",
        "quadrant",
        key("mermaid.builder.point"),
        "$0: [$1, $2]",
        [
            label("New point"),
            coordinate(key("mermaid.builder.x"), "0.5"),
            coordinate(key("mermaid.builder.y"), "0.5")
        ]
    ),
    recipe!("xy.bar", "xy", key("mermaid.builder.bar"), "bar [$0]", [numbers("3, 7, 5")]),
    recipe!("xy.line", "xy", key("mermaid.builder.line"), "line [$0]", [numbers("2, 5, 8")]),
    recipe!("xy.axis", "xy", key("mermaid.builder.axis"), "x-axis [$0]", [numbers("1, 2, 3")]),
    recipe!(
        "radar.curve",
        "radar",
        key("mermaid.builder.curve"),
        "curve $0[$1]{$2}",
        [id("team2"), text("New team"), numbers("5, 8, 7")]
    ),
    recipe!(
        "packet.field",
        "packet",
        key("mermaid.builder.packet_field"),
        "$0-$1: $2",
        [
            field(key("mermaid.builder.first_bit"), FieldKind::Unsigned, "32"),
            field(key("mermaid.builder.last_bit"), FieldKind::Unsigned, "63"),
            text("New field")
        ]
    ),
    recipe!("mindmap.node", "mindmap", key("mermaid.builder.node"), "$0", [label("New idea")]),
    recipe!(
        "git.commit",
        "git",
        key("mermaid.builder.commit"),
        "commit id: $0",
        [text("change1")]
    ),
    recipe!("git.branch", "git", key("mermaid.builder.branch"), "branch $0", [id("feature")]),
    recipe!("git.checkout", "git", key("mermaid.builder.checkout"), "checkout $0", [id("main")]),
    recipe!("git.merge", "git", key("mermaid.builder.merge"), "merge $0", [id("develop")]),
    recipe!(
        "requirement.node",
        "requirement",
        key("mermaid.builder.requirement"),
        "requirement $0 {\n        id: $1\n        text: $2\n        risk: $3\n        verifymethod: $4\n    }",
        [
            id("req1"),
            id("R1"),
            label("New requirement"),
            choice(key("mermaid.builder.risk"), &["low", "medium", "high"], "low"),
            choice(
                key("mermaid.builder.verify"),
                &["test", "analysis", "inspection", "demonstration"],
                "test"
            )
        ]
    ),
    recipe!(
        "requirement.element",
        "requirement",
        key("mermaid.builder.element"),
        "element $0 {\n        type: $1\n    }",
        [id("test1"), label("test")]
    ),
    recipe!(
        "requirement.edge",
        "requirement",
        key("mermaid.builder.relation"),
        "$0 - $1 -> $2",
        [
            from("test"),
            choice(
                key("mermaid.builder.relation"),
                &["contains", "copies", "derives", "satisfies", "verifies", "refines", "traces"],
                "verifies"
            ),
            to("delivery")
        ]
    ),
    recipe!(
        "c4.person",
        "c4_context",
        key("mermaid.builder.person"),
        "Person($0, $1, $2)",
        [id("person1"), name("New person"), description("Description")]
    ),
    recipe!(
        "c4.system",
        "c4_context",
        key("mermaid.builder.system"),
        "System($0, $1, $2)",
        [id("system1"), name("New system"), description("Description")]
    ),
    recipe!(
        "c4.context_edge",
        "c4_context",
        key("mermaid.builder.relation"),
        "Rel($0, $1, $2)",
        [from("user"), to("shop"), text("Uses")]
    ),
    recipe!(
        "c4.container",
        "c4_container",
        key("mermaid.builder.container"),
        "Container($0, $1, $2, $3)",
        [
            id("container1"),
            name("New container"),
            technology("Rust"),
            description("Description")
        ]
    ),
    recipe!(
        "c4.container_edge",
        "c4_container",
        key("mermaid.builder.relation"),
        "Rel($0, $1, $2)",
        [from("app"), to("db"), text("Uses")]
    ),
    recipe!(
        "c4.component",
        "c4_component",
        key("mermaid.builder.component"),
        "Component($0, $1, $2, $3)",
        [
            id("component1"),
            name("New component"),
            technology("Rust"),
            description("Description")
        ]
    ),
    recipe!(
        "c4.component_edge",
        "c4_component",
        key("mermaid.builder.relation"),
        "Rel($0, $1, $2)",
        [from("api"), to("service"), text("Calls")]
    ),
    recipe!(
        "c4.dynamic_edge",
        "c4_dynamic",
        key("mermaid.builder.relation"),
        "Rel($0, $1, $2)",
        [from("app"), to("db"), text("Reads")]
    ),
    recipe!(
        "c4.dynamic_container",
        "c4_dynamic",
        key("mermaid.builder.container"),
        "Container($0, $1, $2, $3)",
        [
            id("container1"),
            name("New container"),
            technology("Rust"),
            description("Description")
        ]
    ),
    recipe!(
        "c4.deployment",
        "c4_deployment",
        key("mermaid.builder.deployment_node"),
        "Deployment_Node($0, $1, $2) {\n    }",
        [id("server2"), name("New server"), technology("Linux")]
    ),
    recipe!(
        "c4.deployed_container",
        "c4_deployment",
        key("mermaid.builder.container"),
        "Container($0, $1, $2, $3)",
        [
            id("container1"),
            name("New container"),
            technology("Rust"),
            description("Description")
        ]
    ),
    recipe!(
        "block.node",
        "block",
        key("mermaid.builder.node"),
        "$0[$1]",
        [id("block1"), text("New block")]
    ),
    recipe!(
        "block.edge",
        "block",
        key("mermaid.builder.relation"),
        "$0 --> $1",
        [from("app"), to("db")]
    ),
    recipe!(
        "architecture.group",
        "architecture",
        key("mermaid.builder.group"),
        "group $0($1)[$2]",
        [
            id("group1"),
            choice(
                key("mermaid.builder.icon"),
                &["cloud", "server", "database", "disk", "internet"],
                "cloud"
            ),
            label("New group")
        ]
    ),
    recipe!(
        "architecture.service",
        "architecture",
        key("mermaid.builder.service"),
        "service $0($1)[$2] in $3",
        [
            id("service1"),
            choice(
                key("mermaid.builder.icon"),
                &["cloud", "server", "database", "disk", "internet"],
                "server"
            ),
            label("New service"),
            id("api")
        ]
    ),
    recipe!(
        "architecture.edge",
        "architecture",
        key("mermaid.builder.relation"),
        "$0:$1 -- $2:$3",
        [
            from("app"),
            choice(key("mermaid.builder.port"), &["L", "R", "T", "B"], "R"),
            choice(key("mermaid.builder.port"), &["L", "R", "T", "B"], "L"),
            to("db")
        ]
    ),
    recipe!(
        "kanban.column",
        "kanban",
        key("mermaid.builder.column"),
        "$0[$1]",
        [id("column1"), label("New column")]
    ),
    recipe!(
        "treemap.value",
        "treemap",
        key("mermaid.builder.data"),
        "$0: $1",
        [text("New category"), number("1", 0.0, f64::MAX)]
    ),
    recipe!("treemap.group", "treemap", key("mermaid.builder.group"), "$0", [text("New group")]),
    recipe!(
        "zenuml.actor",
        "zenuml",
        key("mermaid.builder.participant"),
        "@Actor $0",
        [id("Actor1")]
    ),
    recipe!(
        "zenuml.message",
        "zenuml",
        key("mermaid.builder.message"),
        "$0 -> $1.$2()",
        [from("Customer"), to("Shop"), id("confirm")]
    ),
    recipe!(
        "swimlane.node",
        "swimlane",
        key("mermaid.builder.node"),
        "$0[$1]",
        [id("N1"), text("New step")]
    ),
    recipe!(
        "swimlane.group",
        "swimlane",
        key("mermaid.builder.group"),
        "subgraph $0
    end",
        [id("Team1")]
    ),
    recipe!(
        "swimlane.edge",
        "swimlane",
        key("mermaid.builder.relation"),
        "$0 --> $1",
        [from("choose"), to("deliver")]
    ),
    recipe!(
        "usecase.actor",
        "usecase",
        key("mermaid.builder.participant"),
        "actor $0",
        [id("Actor1")]
    ),
    recipe!(
        "usecase.node",
        "usecase",
        key("mermaid.builder.node"),
        "$0($1)",
        [id("Case1"), text("New use case")]
    ),
    recipe!(
        "usecase.edge",
        "usecase",
        key("mermaid.builder.relation"),
        "$0 --> $1",
        [from("Customer"), to("Checkout")]
    ),
    recipe!(
        "eventmodeling.frame",
        "eventmodeling",
        key("mermaid.builder.event"),
        "tf $0 $1 $2",
        [
            field(key("mermaid.builder.identifier"), FieldKind::Unsigned, "04"),
            choice(key("mermaid.builder.element"), &["ui", "cmd", "evt", "rmo", "pcr"], "evt"),
            id("OrderCompleted")
        ]
    ),
    recipe!("venn.set", "venn", key("mermaid.builder.group"), "set $0", [id("Test")]),
    recipe!(
        "venn.union",
        "venn",
        key("mermaid.builder.relation"),
        "union $0,$1[$2]",
        [from("Design"), to("Build"), text("Shared work")]
    ),
    recipe!(
        "ishikawa.cause",
        "ishikawa",
        key("mermaid.builder.node"),
        "$0",
        [label("New cause")]
    ),
    recipe!(
        "wardley.component",
        "wardley",
        key("mermaid.builder.component"),
        "component $0 [$1, $2]",
        [
            label("New component"),
            coordinate(key("mermaid.builder.visibility"), "0.5"),
            coordinate(key("mermaid.builder.evolution"), "0.5")
        ]
    ),
    recipe!(
        "wardley.edge",
        "wardley",
        key("mermaid.builder.relation"),
        "$0 -> $1",
        [label("Customer"), label("Shop")]
    ),
    recipe!(
        "cynefin.domain",
        "cynefin",
        key("mermaid.builder.group"),
        "$0",
        [choice(
            key("mermaid.builder.group"),
            &["clear", "complicated", "complex", "chaotic", "confusion"],
            "complicated"
        )]
    ),
    recipe!("cynefin.item", "cynefin", key("mermaid.builder.element"), "$0", [text("New item")]),
    recipe!(
        "treeview.node",
        "treeview",
        key("mermaid.builder.node"),
        "$0",
        [label("new-file.rs")]
    ),
];

fn canonical_header(header: &str) -> String {
    let lower = header.to_ascii_lowercase();
    match lower.as_str() {
        "graph" | "flowchart-elk" => "flowchart".into(),
        _ => lower.trim_end_matches("-beta").trim_end_matches("-v2").into(),
    }
}

pub fn family(source: &str) -> Option<&'static Template> {
    let header = canonical_header(super::header_token(source)?);
    TEMPLATES
        .iter()
        .find(|t| canonical_header(super::header_token(t.source).unwrap()) == header)
}

pub fn recipes(source: &str) -> impl Iterator<Item = &'static Recipe> {
    let family = family(source).map(|t| t.id);
    RECIPES.iter().filter(move |recipe| Some(recipe.family) == family)
}

impl FieldKind {
    fn encode(self, value: &str) -> Option<String> {
        match self {
            Self::Identifier => {
                let mut bytes = value.bytes();
                if !bytes.next().is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
                    || !bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    || matches!(value, "end" | "class" | "state" | "graph" | "flowchart" | "subgraph" | "direction")
                {
                    return None;
                }
                Some(value.into())
            }
            Self::Quoted => Some(format!("\"{}\"", conversion::encode(value))),
            Self::Label => (!value.trim().is_empty()
                && !value.contains([
                    '\r', '\n', '"', '\'', ':', ';', ',', '[', ']', '{', '}', '(', ')', '<', '>', '|', '%', '`', '\\',
                ]))
            .then(|| value.into()),
            Self::Number { min, max } => {
                let n: f64 = value.parse().ok()?;
                (n.is_finite() && (min..=max).contains(&n)).then(|| value.into())
            }
            Self::Unsigned => value.parse::<u32>().ok().map(|_| value.into()),
            Self::Numbers => {
                let valid = !value.trim().is_empty() && value.split(',').all(|v| v.trim().parse::<f64>().is_ok_and(f64::is_finite));
                valid.then(|| value.into())
            }
            Self::Choice(options) => options.contains(&value).then(|| value.into()),
            Self::Date => {
                let parts: Vec<_> = value.split('-').collect();
                if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
                    return None;
                }
                let year: u32 = parts[0].parse().ok()?;
                let month: u32 = parts[1].parse().ok()?;
                let day: u32 = parts[2].parse().ok()?;
                let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
                let days = match month {
                    2 => {
                        if leap {
                            29
                        } else {
                            28
                        }
                    }
                    4 | 6 | 9 | 11 => 30,
                    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                    _ => return None,
                };
                (year > 0 && (1..=days).contains(&day)).then(|| value.into())
            }
            Self::Duration => {
                let n: u32 = value.get(..value.len().checked_sub(1)?)?.parse().ok()?;
                (n > 0 && matches!(value.as_bytes().last(), Some(b'd' | b'h' | b'm' | b's' | b'w'))).then(|| value.into())
            }
        }
    }

    fn decode(self, value: &str) -> Option<String> {
        let decoded = match self {
            Self::Quoted => conversion::decode(value.strip_prefix('"')?.strip_suffix('"')?),
            _ => value.into(),
        };
        self.encode(&decoded)?;
        Some(decoded)
    }
}

impl Recipe {
    pub fn defaults(&self) -> Vec<String> {
        self.fields.iter().map(|field| field.default.into()).collect()
    }

    pub fn render(&self, values: &[String]) -> Result<String, String> {
        if values.len() != self.fields.len() {
            return Err(tr("mermaid.builder.invalid_fields").into());
        }
        let input_bytes = values.iter().try_fold(0usize, |total, value| total.checked_add(value.len()));
        if input_bytes.is_none_or(|bytes| bytes > MAX_BUILDER_BYTES) {
            return Err(tr("file.input_limit").into());
        }
        let mut result = String::new();
        let mut remaining = self.pattern;
        for (index, (field, value)) in self.fields.iter().zip(values).enumerate() {
            let (literal, rest) = remaining.split_once(&format!("${index}")).expect("ordered recipe slots");
            result.push_str(literal);
            result.push_str(
                &field
                    .kind
                    .encode(value)
                    .ok_or_else(|| trf("mermaid.builder.invalid_field", &[&tr(field.label)]))?,
            );
            remaining = rest;
        }
        result.push_str(remaining);
        if self.id == "packet.field" {
            let start = values[0].parse::<u32>().unwrap();
            let end = values[1].parse::<u32>().unwrap();
            if start > end || end > 65_535 {
                return Err(tr("mermaid.builder.invalid_fields").into());
            }
        }
        Ok(result)
    }

    fn read(&self, text: &str) -> Option<Vec<String>> {
        let mut remaining_pattern = self.pattern;
        let mut remaining_text = text;
        let mut values = Vec::new();
        for (index, field) in self.fields.iter().enumerate() {
            let (literal, rest) = remaining_pattern.split_once(&format!("${index}"))?;
            remaining_text = remaining_text.strip_prefix(literal)?;
            let next_literal = rest.split('$').next()?;
            let end = if matches!(field.kind, FieldKind::Quoted) {
                // Generated quotes are entity-escaped. Delimiters inside a quoted
                // label (e.g. C4 commas or a state's " as ") remain label text.
                remaining_text.strip_prefix('"')?.find('"')? + 2
            } else if next_literal.is_empty() {
                remaining_text.len()
            } else {
                remaining_text.find(next_literal)?
            };
            values.push(field.kind.decode(&remaining_text[..end])?);
            remaining_text = &remaining_text[end..];
            remaining_pattern = rest;
        }
        (remaining_text == remaining_pattern).then_some(values)
    }
}

/// Recognized statements only. Unknown lines, comments and frontmatter remain in source.
/// This small grammar intentionally avoids attempting to normalize arbitrary Mermaid syntax.
pub fn statements(source: &str) -> Vec<Statement> {
    if source.len() > MAX_BUILDER_BYTES {
        return Vec::new();
    }
    let recipes: Vec<_> = recipes(source).collect();
    let Some(header) = super::header_range(source) else {
        return Vec::new();
    };
    let mut offset = 0;
    let mut result = Vec::new();
    let mut consumed = 0;
    for (index, line) in source.split_inclusive('\n').enumerate() {
        let trimmed = line.trim();
        let start = offset + line.len() - line.trim_start().len();
        offset += line.len();
        if start <= header.end {
            continue;
        }
        if start < consumed || trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        for &recipe in &recipes {
            let count = recipe.pattern.matches('\n').count() + 1;
            let mut end = start;
            for part in source[start..].split_inclusive('\n').take(count) {
                end += part.len();
            }
            let original = source[start..end].trim_end_matches(['\r', '\n']);
            // Preserve CRLF outside the edited statement and use normalized line endings only for matching.
            if let Some(values) = recipe.read(&original.replace("\r\n", "\n")) {
                let end = start + original.len();
                result.push(Statement {
                    range: start..end,
                    line: index + 1,
                    recipe,
                    values,
                    original: original.into(),
                });
                consumed = end;
                break;
            }
        }
    }
    result
}

/// Replace exactly the statement the user selected, preserving all surrounding source bytes.
pub fn replace(diagram: &mut Diagram, statement: &Statement, values: &[String]) -> Result<(), String> {
    if family(&diagram.source).map(|t| t.id) != Some(statement.recipe.family)
        || diagram.source.get(statement.range.clone()) != Some(statement.original.as_str())
    {
        return Err(tr("mermaid.builder.stale").into());
    }
    if values == statement.values {
        return Ok(());
    }
    let mut rendered = statement.recipe.render(values)?;
    if statement.original.contains("\r\n") {
        rendered = rendered.replace('\n', "\r\n");
    }
    diagram.source.replace_range(statement.range.clone(), &rendered);
    Ok(())
}

/// Remove only a selected, recognized statement, leaving all other syntax untouched.
pub fn remove(diagram: &mut Diagram, statement: &Statement) -> Result<(), String> {
    if family(&diagram.source).map(|t| t.id) != Some(statement.recipe.family)
        || diagram.source.get(statement.range.clone()) != Some(statement.original.as_str())
    {
        return Err(tr("mermaid.builder.stale").into());
    }
    let line_start = diagram.source[..statement.range.start].rfind('\n').map_or(0, |i| i + 1);
    let mut end = statement.range.end;
    if diagram.source[end..].starts_with("\r\n") {
        end += 2;
    } else if diagram.source[end..].starts_with('\n') {
        end += 1;
    }
    diagram.source.replace_range(line_start..end, "");
    Ok(())
}

/// Explicit insertion point and indentation let hierarchical notations retain their structure.
/// No existing statement or unknown source is regenerated.
pub fn insert(diagram: &mut Diagram, recipe: &Recipe, values: &[String], before: Option<usize>, indent: usize) -> Result<(), String> {
    if family(&diagram.source).map(|t| t.id) != Some(recipe.family) || diagram.source.len() > MAX_BUILDER_BYTES || indent > 16 {
        return Err(tr("mermaid.builder.invalid_fields").into());
    }
    let position = before.unwrap_or(diagram.source.len());
    if position < super::header_range(&diagram.source).map_or(usize::MAX, |r| r.end)
        || position > diagram.source.len()
        || !diagram.source.is_char_boundary(position)
        || (position != diagram.source.len() && position > 0 && diagram.source.as_bytes()[position - 1] != b'\n')
    {
        return Err(tr("mermaid.builder.stale").into());
    }
    let rendered = recipe.render(values)?;
    let newline = if diagram.source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut addition = String::new();
    if position > 0 && diagram.source.as_bytes()[position - 1] != b'\n' {
        addition.push_str(newline);
    }
    addition.push_str(&" ".repeat(indent * 2));
    addition.push_str(&rendered.replace('\n', newline));
    addition.push_str(newline);
    diagram.source.insert_str(position, &addition);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, DocumentCodec, JsonCodec};

    #[test]
    fn every_catalogue_family_has_forms_and_available_previews_render_generated_statements() {
        let mut ids = std::collections::HashSet::new();
        for template in TEMPLATES {
            assert_eq!(family(template.source).unwrap().id, template.id);
            assert!(recipes(template.source).next().is_some(), "{}", template.id);
        }
        for recipe in RECIPES {
            assert!(ids.insert(recipe.id), "duplicate recipe {}", recipe.id);
            let template = TEMPLATES.iter().find(|t| t.id == recipe.family).unwrap();
            let mut diagram = Diagram {
                name: recipe.id.into(),
                source: template.source.into(),
            };
            let values = recipe.defaults();
            insert(&mut diagram, recipe, &values, None, 2).unwrap();
            if template.preview {
                let svg = super::super::render_svg(&diagram.source).unwrap_or_else(|e| panic!("{}: {e}\n{}", recipe.id, diagram.source));
                assert!(svg.contains("<svg"));
            } else {
                assert!(super::super::render_svg(&diagram.source).is_err());
            }
            let statement = statements(&diagram.source)
                .into_iter()
                .find(|s| s.recipe.id == recipe.id && s.values == values);
            assert!(
                statement.is_some(),
                "{} cannot edit its generated statement\n{}",
                recipe.id,
                diagram.source
            );
            let codec = JsonCodec::new("mermaid");
            let restored = codec
                .decode(&Document::read(codec.encode(&diagram).to_json().as_bytes()).unwrap())
                .unwrap();
            assert_eq!(diagram, restored);
        }
    }

    #[test]
    fn quoted_form_values_can_contain_statement_delimiters() {
        for recipe in RECIPES {
            let template = TEMPLATES.iter().find(|t| t.id == recipe.family).unwrap();
            let mut values = recipe.defaults();
            let mut quoted = false;
            for (field, value) in recipe.fields.iter().zip(&mut values) {
                if matches!(field.kind, FieldKind::Quoted) {
                    *value = "Révisé, as \"quoted\" : ] } & text".into();
                    quoted = true;
                }
            }
            if !quoted {
                continue;
            }
            let mut diagram = Diagram {
                name: "Labels".into(),
                source: template.source.into(),
            };
            insert(&mut diagram, recipe, &values, None, 2).unwrap();
            assert!(
                statements(&diagram.source)
                    .iter()
                    .any(|s| s.recipe.id == recipe.id && s.values == values),
                "{}",
                recipe.id
            );
        }
    }

    #[test]
    fn targeted_edits_preserve_frontmatter_directives_unicode_comments_unknown_syntax_and_crlf() {
        let source = "\u{feff}---\r\ntitle: éàñ\r\n---\r\n%%{init: {theme: 'dark'}}%%\r\nflowchart LR\r\n    %% keep this\r\n    A[\"Original\"]\r\n    B@{ shape: hex, label: 'Other' }\r\n    A --> B\r\n";
        let mut diagram = Diagram {
            name: "Test".into(),
            source: source.into(),
        };
        let statements = statements(source);
        let statement = statements.iter().find(|s| s.recipe.id == "flow.node").unwrap();
        let mut values = statement.values.clone();
        values[1] = "Révisé <script> \" ] --> Injected[bad] #1;\nnext".into();
        replace(&mut diagram, statement, &values).unwrap();
        assert_eq!(&diagram.source[..statement.range.start], &source[..statement.range.start]);
        assert!(diagram.source.ends_with(&source[statement.range.end..]));
        let parsed = mermaid_svg::parse(&diagram.source).unwrap();
        let mermaid_svg::Diagram::Flowchart(graph) = parsed else {
            unreachable!()
        };
        assert_eq!(graph.nodes.len(), 2, "label text must never inject Mermaid declarations");
        let changed = super::statements(&diagram.source)
            .into_iter()
            .find(|s| s.recipe.id == "flow.node")
            .unwrap();
        assert_eq!(changed.values, values);
        let original = diagram.clone();
        assert!(
            replace(&mut diagram, statement, &values).is_err(),
            "stale statement must not overwrite current source"
        );
        assert_eq!(diagram, original);
        let current = super::statements(&diagram.source)
            .into_iter()
            .find(|s| s.recipe.id == "flow.node")
            .unwrap();
        remove(&mut diagram, &current).unwrap();
        assert_eq!(diagram.source, source.replace("    A[\"Original\"]\r\n", ""));
        let before = diagram.clone();
        assert!(remove(&mut diagram, &current).is_err());
        assert_eq!(diagram, before);
    }

    #[test]
    fn multiline_edits_and_hierarchical_insertions_preserve_group_boundaries() {
        let mut diagram = Diagram {
            name: "Requirements".into(),
            source: "requirementDiagram\r\n    %% before\r\n    requirement req1 {\r\n        id: R1\r\n        text: Original\r\n        risk: low\r\n        verifymethod: test\r\n    }\r\n    %% after\r\n".into(),
        };
        let selected = statements(&diagram.source).pop().unwrap();
        let mut values = selected.values.clone();
        values[2] = "Revised requirement".into();
        replace(&mut diagram, &selected, &values).unwrap();
        assert!(diagram.source.contains("text: Revised requirement\r\n"));
        assert!(diagram.source.ends_with("    }\r\n    %% after\r\n"));
        let mut map = Diagram {
            name: "Map".into(),
            source: "mindmap\n  root((Delivery))\n    Design\n    Build\n".into(),
        };
        let recipe = RECIPES.iter().find(|r| r.id == "mindmap.node").unwrap();
        let before = map.source.find("    Build").unwrap();
        insert(&mut map, recipe, &["Review".into()], Some(before), 3).unwrap();
        assert_eq!(map.source, "mindmap\n  root((Delivery))\n    Design\n      Review\n    Build\n");
        super::super::render_svg(&map.source).unwrap();
        let before = map.clone();
        assert!(insert(&mut map, recipe, &recipe.defaults(), Some(2), 2).is_err());
        assert_eq!(map, before);
    }

    #[test]
    fn unchanged_values_keep_exact_encoding_and_frontmatter_is_never_a_statement() {
        let source = "---\nexample: |\n  flowchart\n  A[\"Untouched\"]\n---\nflowchart LR; B --> C\n    A[\"#x41;\"]\n";
        let mut diagram = Diagram {
            name: "Example".into(),
            source: source.into(),
        };
        let statements = statements(source);
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].line, 7);
        assert_eq!(statements[0].values[1], "A");
        replace(&mut diagram, &statements[0], &statements[0].values).unwrap();
        assert_eq!(diagram.source, source);
        let original = diagram.clone();
        let recipe = statements[0].recipe;
        assert!(insert(&mut diagram, recipe, &recipe.defaults(), Some(0), 2).is_err());
        assert_eq!(diagram, original);
        diagram.source = diagram.source.replace("flowchart LR", "block-beta");
        let original = diagram.clone();
        assert!(replace(&mut diagram, &statements[0], &["A".into(), "Changed".into()]).is_err());
        assert_eq!(diagram, original);
    }

    #[test]
    fn form_errors_are_atomic_and_invalid_drafts_are_unchanged() {
        let recipe = RECIPES.iter().find(|r| r.id == "pie.value").unwrap();
        for value in ["NaN", "inf", "-1", "1\nflowchart LR", ""] {
            let mut diagram = Diagram {
                name: "Pie".into(),
                source: "pie\n\"A\": 1\n".into(),
            };
            let original = diagram.clone();
            assert!(insert(&mut diagram, recipe, &["B".into(), value.into()], None, 2).is_err());
            assert_eq!(diagram, original);
        }
        let mut oversized = Diagram {
            name: "Pie".into(),
            source: "pie\n".into(),
        };
        assert!(insert(&mut oversized, recipe, &["A".repeat(MAX_BUILDER_BYTES + 1), "1".into()], None, 2).is_err());
        assert_eq!(oversized.source, "pie\n");
        for date in ["2026-02-29", "2026-13-01", "2026-01-32", "2026-1-01", "0000-01-01"] {
            assert!(FieldKind::Date.encode(date).is_none(), "{date}");
        }
        assert!(FieldKind::Date.encode("2028-02-29").is_some());
        let mut empty = Diagram {
            name: "Empty".into(),
            source: "pie".into(),
        };
        insert(&mut empty, recipe, &recipe.defaults(), None, 2).unwrap();
        super::super::render_svg(&empty.source).unwrap();
        assert!(FieldKind::Identifier.encode("A\nB").is_none());
        assert!(FieldKind::Identifier.encode("end").is_none());
        assert!(FieldKind::Numbers.encode("1, NaN").is_none());
        let packet = RECIPES.iter().find(|r| r.id == "packet.field").unwrap();
        for values in [["3.5", "5", "Field"], ["8", "7", "Field"], ["0", "65536", "Field"]] {
            assert!(packet.render(&values.map(String::from)).is_err());
        }
        for source in ["not a diagram", "flowchart LR\n A[unfinished", "%% comment\n"] {
            assert!(statements(source).is_empty());
            let diagram = Diagram {
                name: "Draft".into(),
                source: source.into(),
            };
            let codec = JsonCodec::new("mermaid");
            assert_eq!(codec.decode(&codec.encode(&diagram)).unwrap(), diagram);
        }
    }
}
