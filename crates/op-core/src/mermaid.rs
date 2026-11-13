//! Text-first Mermaid documents. Invalid drafts remain editable and serializable.
use crate::i18n::key;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub name: String,
    pub source: String,
}

pub struct Template {
    pub id: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "mermaid.flowchart",
        label: key("mermaid.flowchart"),
        source: "flowchart TD\n    A[Start] --> B{Approved?}\n    B -->|Yes| C[Process order]\n    B -->|No| D[Review]\n    D --> B\n    C --> E[End]\n",
    },
    Template {
        id: "mermaid.sequence",
        label: key("mermaid.sequence"),
        source: "sequenceDiagram\n    participant User\n    participant App\n    participant Database\n    User->>App: Submit request\n    App->>Database: Save\n    Database-->>App: Confirmation\n    App-->>User: Done\n",
    },
    Template {
        id: "mermaid.class",
        label: key("mermaid.class"),
        source: "classDiagram\n    class Order {\n        +int id\n        +confirm()\n    }\n    class Item {\n        +String name\n        +int quantity\n    }\n    Order \"1\" *-- \"many\" Item : contains\n",
    },
    Template {
        id: "mermaid.state",
        label: key("mermaid.state"),
        source: "stateDiagram-v2\n    [*] --> Draft\n    Draft --> Review: Submit\n    Review --> Approved: Accept\n    Review --> Draft: Revise\n    Approved --> [*]\n",
    },
    Template {
        id: "mermaid.er",
        label: key("mermaid.er"),
        source: "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    CUSTOMER {\n        int id PK\n        string name\n    }\n    ORDER {\n        int id PK\n        int customer_id FK\n    }\n",
    },
];

impl Diagram {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: TEMPLATES[0].source.into(),
        }
    }
}

/// Content sniffing only: no parsing/layout on the UI thread or while importing.
/// A recognized but invalid draft must open so its source can be corrected.
pub fn accepts(source: &str) -> bool {
    let source = source.trim_start_matches('\u{feff}').trim_start();
    let mut lines = source.lines().map(str::trim).filter(|l| !l.is_empty());
    // Mermaid YAML frontmatter is preserved verbatim.
    if source.starts_with("---\n") || source.starts_with("---\r\n") {
        lines.next();
        for line in lines.by_ref() {
            if line == "---" {
                break;
            }
        }
    }
    let mut directive = false;
    let Some(header) = lines.find(|line| {
        if directive {
            directive = !line.contains("}%%");
            return false;
        }
        if line.starts_with("%%{") {
            directive = !line.contains("}%%");
            return false;
        }
        !line.starts_with("%%")
    }) else {
        return false;
    };
    let token = header.split(|c: char| c.is_whitespace() || c == ';').next().unwrap_or("");
    matches!(
        token.to_ascii_lowercase().as_str(),
        "flowchart"
            | "graph"
            | "sequencediagram"
            | "classdiagram"
            | "statediagram"
            | "statediagram-v2"
            | "erdiagram"
            | "pie"
            | "xychart"
            | "xychart-beta"
            | "quadrantchart"
            | "gantt"
            | "timeline"
            | "journey"
            | "mindmap"
            | "gitgraph"
            | "sankey-beta"
            | "sankey"
            | "kanban"
            | "block-beta"
            | "block"
            | "packet-beta"
            | "packet"
            | "architecture-beta"
            | "architecture"
            | "radar-beta"
            | "radar"
            | "treemap-beta"
            | "treemap"
            | "c4context"
            | "c4container"
            | "c4component"
            | "c4dynamic"
            | "c4deployment"
            | "requirementdiagram"
            | "zenuml"
    )
}

pub fn render_svg(source: &str) -> Result<String, String> {
    // Keep rendering deterministic and independent of OS clocks (unavailable in Wasm).
    // The source remains untouched; only the derived Gantt rendering omits today's marker.
    let diagram = mermaid_svg::parse(source).map_err(|e| e.to_string())?;
    if matches!(diagram, mermaid_svg::Diagram::Gantt(_)) {
        return mermaid_svg::render(&format!("{source}\ntodayMarker off\n")).map_err(|e| e.to_string());
    }
    mermaid_svg::render(source).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, DocumentCodec, JsonCodec, ReadError};

    #[test]
    fn templates_produce_svg() {
        for t in TEMPLATES {
            assert!(accepts(t.source), "{}", t.id);
            let svg = render_svg(t.source).unwrap_or_else(|e| panic!("{}: {e}", t.id));
            assert!(svg.contains("<svg") && svg.contains("<text"), "{}", t.id);
        }
        assert!(render_svg("not a diagram").is_err());
        let gantt = "gantt\n dateFormat YYYY-MM-DD\n section Work\n Task :2026-11-14, 2d\n";
        assert_eq!(render_svg(gantt).unwrap(), render_svg(gantt).unwrap());
    }

    #[test]
    fn drafts_roundtrip_without_changing_the_envelope_or_accepting_future_versions() {
        let codec = JsonCodec::new("mermaid");
        let draft = Diagram {
            name: "Essai éàñ".into(),
            source: "flowchart TD\n A[unfinished".into(),
        };
        let doc = Document::read(codec.encode(&draft).to_json().as_bytes()).unwrap();
        assert_eq!(draft, codec.decode(&doc).unwrap());
        assert!(matches!(
            codec.decode(&Document { model_version: 2, ..doc }),
            Err(ReadError::Unsupported(_))
        ));
    }

    #[test]
    fn detection_is_specific_and_preserves_comments_frontmatter_and_invalid_drafts() {
        assert!(accepts("\u{feff}  %% comment\nflowchart LR; A-->B"));
        assert!(accepts("---\r\ntitle: Demo\r\n---\r\n%% comment\r\nsequenceDiagram\n broken"));
        assert!(accepts("flowchart TD\n A["));
        assert!(accepts("%%{\ninit: {theme:'dark'}\n}%%\nflowchart TD\n A-->B"));
        assert!(accepts("radar-beta\n title Scores"));
        assert!(accepts("treemap-beta\n Root"));
        for text in ["", "A B C", "graphical design", "sequenceDiagrammed", "<log/>", "%% comment"] {
            assert!(!accepts(text), "{text}");
        }
    }
}
