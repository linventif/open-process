//! Tiny XML DOM (namespace prefixes stripped) and a pretty-printing writer.

use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub const MAX_XML_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 128;
const MAX_ELEMENTS: usize = 100_000;
const MAX_ATTRIBUTES: usize = 128;

#[derive(Clone, Debug, Default)]
pub struct Element {
    /// Local name (namespace prefix removed).
    pub name: String,
    /// Attributes with local names.
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn attr_f32(&self, name: &str) -> Option<f32> {
        self.attr(name).and_then(|v| v.trim().parse().ok())
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// All descendants (depth-first, excluding self).
    pub fn descendants(&self) -> Vec<&Element> {
        let mut out = Vec::new();
        let mut stack: Vec<&Element> = self.children.iter().rev().collect();
        while let Some(e) = stack.pop() {
            out.push(e);
            stack.extend(e.children.iter().rev());
        }
        out
    }

    pub fn find(&self, name: &str) -> Option<&Element> {
        self.descendants().into_iter().find(|e| e.name == name)
    }

    /// Text content of a child path such as `["name", "text"]`.
    pub fn text_at(&self, path: &[&str]) -> Option<&str> {
        let mut cur = self;
        for p in path {
            cur = cur.child(p)?;
        }
        Some(cur.text.trim())
    }
}

fn local(name: &str) -> String {
    match name.rsplit_once(':') {
        Some((_, l)) => l.to_string(),
        None => name.to_string(),
    }
}

/// Detects an XML format from its root without building a DOM or assuming a prefix.
pub fn root_name(src: &str) -> Option<String> {
    if src.len() > MAX_XML_BYTES {
        return None;
    }
    let mut reader = Reader::from_str(src);
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => return Some(local(e.name().as_ref())),
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn start_element(e: &quick_xml::events::BytesStart<'_>) -> Result<Element, String> {
    let mut el = Element {
        name: local(e.name().as_ref()),
        ..Default::default()
    };
    for (index, a) in e.attributes().enumerate() {
        if index >= MAX_ATTRIBUTES {
            return Err(crate::i18n::tr("xml.resource_limit").into());
        }
        let a = a.map_err(|e| e.to_string())?;
        let key = local(a.key.as_ref());
        let val = a.normalized_value(XmlVersion::Implicit1_0).map_err(|e| e.to_string())?;
        el.attrs.push((key, val.into_owned()));
    }
    Ok(el)
}

/// Parses an XML document and returns its root element.
pub fn parse(s: &str) -> Result<Element, String> {
    if s.len() > MAX_XML_BYTES {
        return Err(crate::i18n::tr("xml.resource_limit").into());
    }
    let mut reader = Reader::from_str(s);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Element> = vec![Element::default()];
    let mut elements = 0;
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("XML error at {}: {e}", reader.buffer_position()))?
        {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                elements += 1;
                if elements > MAX_ELEMENTS || stack.len() > MAX_DEPTH {
                    return Err(crate::i18n::tr("xml.resource_limit").into());
                }
                if stack.len() == 1 && !stack[0].children.is_empty() {
                    return Err(crate::i18n::tr("xml.invalid_structure").into());
                }
                let empty = matches!(event, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = event else {
                    unreachable!()
                };
                let el = start_element(&e)?;
                if empty {
                    stack.last_mut().unwrap().children.push(el);
                } else {
                    stack.push(el);
                }
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err(crate::i18n::tr("xml.invalid_structure").into());
                }
                let el = stack.pop().ok_or(crate::i18n::tr("xml.invalid_structure"))?;
                stack.last_mut().ok_or(crate::i18n::tr("xml.invalid_structure"))?.children.push(el);
            }
            Event::Text(t) => {
                let text = t.xml10_content();
                if stack.len() == 1 && !text.trim().is_empty() {
                    return Err(crate::i18n::tr("xml.invalid_structure").into());
                }
                stack.last_mut().unwrap().text.push_str(&text);
            }
            Event::CData(t) => {
                if stack.len() == 1 {
                    return Err(crate::i18n::tr("xml.invalid_structure").into());
                }
                stack.last_mut().unwrap().text.push_str(&t.xml10_content());
            }
            Event::GeneralRef(r) => {
                if stack.len() == 1 {
                    return Err(crate::i18n::tr("xml.invalid_structure").into());
                }
                let txt = if r.is_char_ref() {
                    r.resolve_char_ref()
                        .map_err(|e| e.to_string())?
                        .map(String::from)
                        .ok_or(crate::i18n::tr("xml.invalid_reference"))?
                } else {
                    match &*r {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err(crate::i18n::tr("xml.unsupported_entity").into()),
                    }
                    .to_string()
                };
                stack.last_mut().unwrap().text.push_str(&txt);
            }
            Event::DocType(_) => return Err(crate::i18n::tr("xml.unsupported_entity").into()),
            Event::Eof => break,
            _ => {}
        }
    }
    let mut root = stack.pop().ok_or(crate::i18n::tr("xml.invalid_structure"))?;
    if !stack.is_empty() {
        return Err(crate::i18n::tr("xml.invalid_structure").into());
    }
    root.children
        .pop()
        .ok_or_else(|| crate::i18n::tr("xml.invalid_structure").to_string())
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            _ => out.push(c),
        }
    }
    out
}

/// Minimal indenting XML writer.
#[derive(Default)]
pub struct XmlWriter {
    pub out: String,
    depth: usize,
}

impl XmlWriter {
    pub fn new() -> Self {
        let mut w = Self::default();
        w.out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        w
    }

    fn tag(&mut self, name: &str, attrs: &[(&str, String)], close: &str) {
        self.out.push_str(&"  ".repeat(self.depth));
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push_str(&format!(" {k}=\"{}\"", escape(v)));
        }
        self.out.push_str(close);
        self.out.push('\n');
    }

    pub fn open(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.tag(name, attrs, ">");
        self.depth += 1;
    }

    pub fn empty(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.tag(name, attrs, "/>");
    }

    pub fn close(&mut self, name: &str) {
        self.depth -= 1;
        self.out.push_str(&"  ".repeat(self.depth));
        self.out.push_str(&format!("</{name}>\n"));
    }

    pub fn text_element(&mut self, name: &str, attrs: &[(&str, String)], text: &str) {
        self.out.push_str(&"  ".repeat(self.depth));
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push_str(&format!(" {k}=\"{}\"", escape(v)));
        }
        self.out.push_str(&format!(">{}</{name}>\n", escape(text)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut w = XmlWriter::new();
        w.open("bpmn:root", &[("a", "x & \"y\"".into())]);
        w.text_element("bpmn:name", &[], "a < b");
        w.empty("bpmn:leaf", &[("id", "1".into())]);
        w.close("bpmn:root");
        let r = parse(&w.out).unwrap();
        assert_eq!(r.name, "root");
        assert_eq!(r.attr("a"), Some("x & \"y\""));
        assert_eq!(r.text_at(&["name"]), Some("a < b"));
        assert_eq!(r.child("leaf").unwrap().attr("id"), Some("1"));
    }

    #[test]
    fn rejects_deep_or_oversized_xml_before_building_an_unsafe_tree() {
        let deep = format!("<pnml><net>{}{}</net></pnml>", "<x>".repeat(50_000), "</x>".repeat(50_000));
        assert!(crate::pnml::import(&deep).is_err());
        assert!(parse(&format!("<root>{}</root>", "<x/>".repeat(MAX_ELEMENTS))).is_err());
        assert!(parse(&" ".repeat(MAX_XML_BYTES + 1)).is_err());
        let allowed = format!("{}{}", "<x>".repeat(MAX_DEPTH), "</x>".repeat(MAX_DEPTH));
        assert!(parse(&allowed).is_ok());
        let attributes = (0..=MAX_ATTRIBUTES).map(|n| format!(" a{n}=\"x\"")).collect::<String>();
        assert!(parse(&format!("<root{attributes}/>")).is_err());
    }

    #[test]
    fn rejects_multiple_roots_external_text_and_custom_entities() {
        for source in [
            "<first/><second/>",
            "before<root/>",
            "<root/>after",
            "<root/>&amp;",
            "<root>&unknown;</root>",
            "<!DOCTYPE root><root/>",
            "<root>",
            "</root>",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
        assert_eq!(parse(" <!-- comment --> <root>&#65;&amp;</root> \n").unwrap().text, "A&");
    }
}
