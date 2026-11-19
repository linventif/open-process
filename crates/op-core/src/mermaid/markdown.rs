//! Single-diagram Markdown interchange, without parsing or rendering on import.
use super::Diagram;
use std::fmt::Write;

fn text(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '\n' | '\r' => out.push(' '),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if c.is_ascii_punctuation() => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

pub fn export(diagram: &Diagram, warnings: &[String]) -> String {
    let source = super::export::source(diagram, warnings);
    // A draft may contain code fences: choose a delimiter that cannot close early.
    let longest = source.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(3.max(longest + 1));
    let mut out = format!("# {}\n\n", text(&diagram.name));
    for warning in warnings {
        writeln!(out, "- {}", text(warning)).unwrap();
    }
    if !warnings.is_empty() {
        out.push('\n');
    }
    write!(out, "{fence}mermaid\n{source}\n{fence}\n").unwrap();
    out
}

fn scan(markdown: &str) -> (usize, Option<std::ops::Range<usize>>) {
    let mut open: Option<(u8, usize, usize, bool, bool)> = None;
    let mut result = None;
    let mut count_mermaid = 0;
    let mut offset = 0;
    for line in markdown.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let trimmed = content.trim_start_matches(' ');
        let indent = content.len() - trimmed.len();
        let marker = trimmed.as_bytes().first().copied().unwrap_or(0);
        let count = trimmed.bytes().take_while(|&c| c == marker).count();
        if indent <= 3 && matches!(marker, b'`' | b'~') && count >= 3 {
            let info = trimmed[count..].trim();
            if let Some((start_marker, length, start, mermaid, crlf)) = open {
                if marker == start_marker && count >= length && info.is_empty() {
                    if mermaid {
                        let raw = &markdown[start..offset];
                        let raw = raw.strip_suffix(if crlf { "\r\n" } else { "\n" }).unwrap_or(raw);
                        result = Some(start..start + raw.len());
                    }
                    open = None;
                }
            } else {
                let mermaid = info.eq_ignore_ascii_case("mermaid");
                count_mermaid += usize::from(mermaid);
                open = Some((marker, count, offset + line.len(), mermaid, line.ends_with("\r\n")));
            }
        }
        offset += line.len();
    }
    if open.is_some_and(|(_, _, _, mermaid, _)| mermaid) {
        result = None;
    }
    (count_mermaid, result)
}

/// Explicit Mermaid fences identify the format even for incomplete or ambiguous drafts.
pub fn accepts(markdown: &str) -> bool {
    scan(markdown).0 > 0
}

/// Return exactly one complete Mermaid block; never silently choose among diagrams.
pub fn source(markdown: &str) -> Option<String> {
    let (count, range) = scan(markdown);
    if count != 1 {
        return None;
    }
    range.map(|range| markdown[range].to_owned())
}
