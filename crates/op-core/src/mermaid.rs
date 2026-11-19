//! Text-first Mermaid documents. Invalid drafts remain editable and serializable.
use serde::{Deserialize, Serialize};

pub mod builder;
pub mod conversion;
pub mod export;
pub mod markdown;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub name: String,
    pub source: String,
}

mod catalogue;
pub use catalogue::{TEMPLATES, Template};

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
fn header_range(source: &str) -> Option<std::ops::Range<usize>> {
    let trimmed = source.trim_start_matches('\u{feff}').trim_start();
    let mut offset = source.len() - trimmed.len();
    let mut frontmatter = trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n");
    let mut first = true;
    let mut directive = false;
    for line in trimmed.split_inclusive('\n') {
        let text = line.trim();
        let start = offset + line.len() - line.trim_start().len();
        offset += line.len();
        if frontmatter {
            if !first && text == "---" {
                frontmatter = false;
            }
            first = false;
            continue;
        }
        if directive {
            directive = !text.contains("}%%");
            continue;
        }
        if text.starts_with("%%{") {
            directive = !text.contains("}%%");
            continue;
        }
        if text.is_empty() || text.starts_with("%%") {
            continue;
        }
        let token = text.split(|c: char| c.is_whitespace() || c == ';').next()?;
        return Some(start..start + token.len());
    }
    None
}

fn header_token(source: &str) -> Option<&str> {
    header_range(source).map(|range| &source[range])
}

pub fn is_class_source(source: &str) -> bool {
    header_token(source).is_some_and(|token| matches!(token, "classDiagram" | "classDiagram-v2"))
}

pub fn accepts(source: &str) -> bool {
    builder::family(source).is_some()
}

pub fn render_svg(source: &str) -> Result<String, String> {
    if builder::family(source).is_some_and(|template| !template.preview) {
        return Err(crate::i18n::tr("mermaid.preview_unsupported").into());
    }
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
            if !t.preview {
                assert!(render_svg(t.source).is_err(), "{} must report the missing renderer", t.id);
                continue;
            }
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
