//! Portable SVG export; text is escaped and kept as editable SVG text.
use super::*;
use crate::xml::escape;
use std::fmt::Write;

fn color(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}
pub fn export(board: &Board) -> Result<String, String> {
    board.validate()?;
    let geometry = board.geometry();
    let b = board.bounds().unwrap_or(Rect::new(0.0, 0.0, 640.0, 480.0));
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\"><title>{}</title>\n",
        b.x - 32.0,
        b.y - 32.0,
        b.w + 64.0,
        b.h + 64.0,
        escape(&board.name)
    );
    for e in &board.elements {
        let r = geometry.element_bounds(e);
        let stroke = color(e.paint.stroke);
        let fill = e.paint.fill.map(color).unwrap_or_else(|| "none".into());
        let attrs = format!(
            "stroke=\"{stroke}\" fill=\"{fill}\" stroke-width=\"{}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"",
            e.paint.width
        );
        match e.kind {
            Kind::Rectangle | Kind::Note => {
                writeln!(
                    svg,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" {attrs}/>",
                    r.x, r.y, r.w, r.h
                )
                .unwrap();
            }
            Kind::Ellipse => {
                writeln!(
                    svg,
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" {attrs}/>",
                    r.center().x,
                    r.center().y,
                    r.w / 2.0,
                    r.h / 2.0
                )
                .unwrap();
            }
            Kind::Diamond => {
                writeln!(
                    svg,
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" {attrs}/>",
                    r.center().x,
                    r.y,
                    r.right(),
                    r.center().y,
                    r.center().x,
                    r.bottom(),
                    r.x,
                    r.center().y
                )
                .unwrap();
            }
            Kind::Line | Kind::Arrow | Kind::Freehand => {
                let points = geometry.points(e);
                let coords = points.iter().map(|p| format!("{},{}", p.x, p.y)).collect::<Vec<_>>().join(" ");
                writeln!(svg,"<polyline points=\"{coords}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",e.paint.width).unwrap();
                if e.kind == Kind::Arrow {
                    let tip = points[points.len() - 1];
                    let from = points[points.len() - 2];
                    let len = tip.dist(from);
                    if len > 0.01 {
                        let size = (len / 3.0).min(16.0);
                        let dx = (tip.x - from.x) / len;
                        let dy = (tip.y - from.y) / len;
                        writeln!(svg,"<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{}\" stroke-linecap=\"round\"/>",tip.x-dx*size-dy*size*0.5,tip.y-dy*size+dx*size*0.5,tip.x,tip.y,tip.x-dx*size+dy*size*0.5,tip.y-dy*size-dx*size*0.5,e.paint.width).unwrap();
                    }
                }
            }
            Kind::Text => {}
        }
        if !e.text.is_empty() {
            let lines: Vec<_> = e.text.lines().collect();
            let height = lines.len() as f32 * e.paint.font_size * 1.25;
            writeln!(
                svg,
                "<text fill=\"{stroke}\" font-family=\"sans-serif\" font-size=\"{}\" text-anchor=\"middle\">",
                e.paint.font_size
            )
            .unwrap();
            for (i, line) in lines.iter().enumerate() {
                writeln!(
                    svg,
                    "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                    r.center().x,
                    r.center().y - height / 2.0 + e.paint.font_size * (1.0 + i as f32 * 1.25),
                    escape(line)
                )
                .unwrap();
            }
            svg.push_str("</text>\n");
        }
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_export_is_valid_xml_and_escapes_user_text() {
        let mut b = example();
        b.name = "<script> & \"title\"".into();
        b.elements[0].text = "A < B & C\nsecond line".into();
        for kind in Kind::ALL {
            b.add(Element::new(kind, Pos::new(1.0, 2.0), Pos::new(20.0, 30.0), Paint::default()));
        }
        let svg = export(&b).unwrap();
        assert!(svg.contains("A &lt; B &amp; C"));
        assert!(!svg.contains("<script>"));
        assert_eq!(crate::xml::parse(&svg).unwrap().name, "svg");
        b.elements[0].paint.width = -2.0;
        assert!(export(&b).is_err());
    }
}
