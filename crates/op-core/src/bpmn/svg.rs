//! Standalone vector rendering of the complete BPMN diagram in world coordinates.
use super::*;
use crate::xml::escape;
use std::fmt::Write;

const INK: &str = "#243342";
const FONT: f32 = 12.0;

struct Label {
    source: String,
    lines: Vec<String>,
    center: Pos,
    width: f32,
    size: f32,
    rotated: bool,
}

impl Label {
    fn new(source: &str, center: Pos, width: f32, size: f32, rotated: bool) -> Self {
        let columns = (width / (size * 0.65)).floor().max(1.0) as usize;
        let mut lines = Vec::new();
        for paragraph in source.split('\n') {
            let mut line = String::new();
            for token in paragraph.split_inclusive(char::is_whitespace) {
                if line.chars().count() + token.chars().count() > columns && !line.is_empty() {
                    lines.push(line.trim_end().to_string());
                    line.clear();
                }
                for character in token.chars() {
                    if line.chars().count() == columns {
                        lines.push(std::mem::take(&mut line));
                    }
                    line.push(character);
                }
            }
            lines.push(line.trim_end().to_string());
        }
        Self {
            source: source.into(),
            lines,
            center,
            width,
            size,
            rotated,
        }
    }

    fn height(&self) -> f32 {
        self.lines.len() as f32 * self.size * 1.3
    }

    fn bounds(&self) -> Rect {
        let (width, height) = if self.rotated {
            (self.height(), self.width)
        } else {
            (self.width, self.height())
        };
        Rect::from_center(self.center, width, height)
    }

    fn draw(&self, svg: &mut String) {
        if self.source.is_empty() {
            return;
        }
        let transform = if self.rotated {
            format!(" transform=\"rotate(-90 {} {})\"", self.center.x, self.center.y)
        } else {
            String::new()
        };
        writeln!(
            svg,
            "<text font-size=\"{}\" text-anchor=\"middle\" fill=\"{INK}\" stroke=\"none\"{transform}><title>{}</title>",
            self.size,
            escape(&self.source)
        )
        .unwrap();
        for (index, line) in self.lines.iter().enumerate() {
            let y = self.center.y - self.height() / 2.0 + self.size * (1.0 + index as f32 * 1.3);
            writeln!(svg, "<tspan x=\"{}\" y=\"{y}\">{}</tspan>", self.center.x, escape(line)).unwrap();
        }
        svg.push_str("</text>\n");
    }
}

fn midpoint(points: &[Pos]) -> Pos {
    let total: f32 = points.windows(2).map(|segment| segment[0].dist(segment[1])).sum();
    let mut remaining = total / 2.0;
    for segment in points.windows(2) {
        let length = segment[0].dist(segment[1]);
        if remaining <= length && length > f32::EPSILON {
            return segment[0].lerp(segment[1], remaining / length);
        }
        remaining -= length;
    }
    points.first().copied().unwrap_or_default()
}

fn node_label(node: &Node) -> Label {
    let rect = node.rect();
    match node.kind {
        NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::TextAnnotation => {
            Label::new(&node.name, node.pos, (rect.w - 16.0).max(20.0), FONT, false)
        }
        _ => {
            let mut label = Label::new(&node.name, node.pos, 160.0, FONT, false);
            label.center.y = rect.bottom() + 6.0 + label.height() / 2.0;
            label
        }
    }
}

impl BpmnDiagram {
    /// Export actual BPMN shapes and authored text, independently of the viewport.
    /// Geometry and model content are never changed by rendering.
    pub fn to_svg(&self) -> String {
        let routes: Vec<_> = self.flows.iter().map(|flow| route(self, flow)).collect();
        let node_labels: Vec<_> = self.nodes.iter().map(node_label).collect();
        let flow_labels: Vec<_> = self
            .flows
            .iter()
            .zip(&routes)
            .map(|(flow, points)| {
                let mut label = Label::new(&flow.name, midpoint(points), 180.0, FONT - 1.0, false);
                label.center.y -= 8.0 + label.height() / 2.0;
                label
            })
            .collect();
        let mut pool_labels = Vec::new();
        for pool in &self.pools {
            pool_labels.push(Label::new(
                &pool.name,
                Pos::new(pool.pos.x + POOL_HEADER / 2.0, pool.pos.y + pool.height() / 2.0),
                (pool.height() - 16.0).max(20.0),
                FONT,
                true,
            ));
            if pool.lanes.len() > 1 {
                for (index, lane) in pool.lanes.iter().enumerate() {
                    let rect = pool.lane_rect(index);
                    pool_labels.push(Label::new(
                        &lane.name,
                        Pos::new(rect.x + LANE_HEADER / 2.0, rect.center().y),
                        (rect.h - 12.0).max(20.0),
                        FONT,
                        true,
                    ));
                }
            }
        }
        let mut bounds: Option<Rect> = None;
        let mut include = |rect: Rect| {
            bounds = Some(match bounds {
                None => rect,
                Some(old) => Rect::new(
                    old.x.min(rect.x),
                    old.y.min(rect.y),
                    old.right().max(rect.right()) - old.x.min(rect.x),
                    old.bottom().max(rect.bottom()) - old.y.min(rect.y),
                ),
            });
        };
        for pool in &self.pools {
            include(pool.rect());
        }
        for node in &self.nodes {
            include(node.rect());
        }
        for point in routes.iter().flatten() {
            include(Rect::new(point.x - 10.0, point.y - 10.0, 20.0, 20.0));
        }
        for label in node_labels
            .iter()
            .chain(&flow_labels)
            .chain(&pool_labels)
            .filter(|label| !label.source.is_empty())
        {
            include(label.bounds());
        }
        let bounds = bounds.unwrap_or(Rect::new(0.0, 0.0, 640.0, 480.0));
        let view = Rect::new(bounds.x - 24.0, bounds.y - 24.0, bounds.w + 48.0, bounds.h + 48.0);
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\"><title>{}</title>\n",
            view.x,
            view.y,
            view.w,
            view.h,
            view.w,
            view.h,
            escape(&self.name)
        );
        svg.push_str("<defs><marker id=\"bpmn-sequence\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" markerWidth=\"10\" markerHeight=\"10\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1 1L11 6L1 11Z\" fill=\"#243342\"/></marker><marker id=\"bpmn-open\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" markerWidth=\"10\" markerHeight=\"10\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1 1L11 6L1 11\" stroke=\"#243342\" fill=\"none\"/></marker><marker id=\"bpmn-message\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" markerWidth=\"11\" markerHeight=\"11\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1 1L11 6L1 11Z\" stroke=\"#243342\" fill=\"white\"/></marker></defs>\n");
        writeln!(svg, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\"/><g font-family=\"sans-serif\" stroke=\"{INK}\" stroke-width=\"1.4\" fill=\"white\" stroke-linejoin=\"round\" stroke-linecap=\"round\">", view.x, view.y, view.w, view.h).unwrap();
        for pool in &self.pools {
            draw_pool(&mut svg, pool);
        }
        for ((flow, points), label) in self.flows.iter().zip(&routes).zip(&flow_labels) {
            if points.len() < 2 {
                continue;
            }
            writeln!(
                svg,
                "<g id=\"flow{}\" data-name=\"{}\"><title>{}</title>",
                flow.id,
                escape(&flow.name),
                escape(&flow.name)
            )
            .unwrap();
            let data = self
                .node(flow.from)
                .into_iter()
                .chain(self.node(flow.to))
                .any(|node| matches!(node.kind, NodeKind::DataObject | NodeKind::DataStore));
            let attrs = match flow.kind {
                FlowKind::Sequence => "marker-end=\"url(#bpmn-sequence)\"",
                FlowKind::Message => "stroke-dasharray=\"7 4\" marker-end=\"url(#bpmn-message)\"",
                FlowKind::Association if data => "stroke-dasharray=\"2 3\" marker-end=\"url(#bpmn-open)\"",
                FlowKind::Association => "stroke-dasharray=\"2 3\"",
            };
            writeln!(
                svg,
                "<polyline points=\"{}\" fill=\"none\" {attrs}/>",
                points
                    .iter()
                    .map(|point| format!("{},{}", point.x, point.y))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .unwrap();
            if flow.kind == FlowKind::Message {
                circle(&mut svg, points[0], 3.0, "fill=\"white\"");
            }
            if flow.is_default && flow.kind == FlowKind::Sequence {
                let delta = points[1] - points[0];
                let length = points[0].dist(points[1]);
                if length > f32::EPSILON {
                    let (dx, dy) = (delta.x / length, delta.y / length);
                    let center = Pos::new(points[0].x + 10.0 * dx, points[0].y + 10.0 * dy);
                    line(
                        &mut svg,
                        Pos::new(center.x - 5.0 * (dy + dx), center.y + 5.0 * (dx - dy)),
                        Pos::new(center.x + 5.0 * (dy + dx), center.y - 5.0 * (dx - dy)),
                        "",
                    );
                }
            }
            label.draw(&mut svg);
            svg.push_str("</g>\n");
        }
        for (node, label) in self.nodes.iter().zip(&node_labels) {
            writeln!(
                svg,
                "<g id=\"node{}\" data-name=\"{}\"><title>{}</title><desc>{}</desc>",
                node.id,
                escape(&node.name),
                escape(&node.name),
                escape(&node.documentation)
            )
            .unwrap();
            draw_node(&mut svg, node);
            label.draw(&mut svg);
            svg.push_str("</g>\n");
        }
        for label in &pool_labels {
            label.draw(&mut svg);
        }
        svg.push_str("</g></svg>\n");
        svg
    }
}

fn line(svg: &mut String, from: Pos, to: Pos, attrs: &str) {
    writeln!(svg, "<path d=\"M{} {}L{} {}\" fill=\"none\" {attrs}/>", from.x, from.y, to.x, to.y).unwrap();
}

fn circle(svg: &mut String, center: Pos, radius: f32, attrs: &str) {
    writeln!(svg, "<circle cx=\"{}\" cy=\"{}\" r=\"{radius}\" {attrs}/>", center.x, center.y).unwrap();
}

fn rectangle(svg: &mut String, rect: Rect, rounded: bool, attrs: &str) {
    writeln!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" {attrs}/>",
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if rounded { 9.0 } else { 0.0 }
    )
    .unwrap();
}

fn draw_pool(svg: &mut String, pool: &Pool) {
    writeln!(svg, "<g id=\"pool{}\" data-name=\"{}\">", pool.id, escape(&pool.name)).unwrap();
    rectangle(svg, pool.rect(), false, "");
    rectangle(
        svg,
        Rect::new(pool.pos.x, pool.pos.y, POOL_HEADER, pool.height()),
        false,
        "fill=\"#edf2f6\"",
    );
    if pool.lanes.len() > 1 {
        for (index, lane) in pool.lanes.iter().enumerate() {
            let rect = pool.lane_rect(index);
            writeln!(svg, "<g id=\"lane{}\" data-name=\"{}\">", lane.id, escape(&lane.name)).unwrap();
            rectangle(svg, Rect::new(rect.x, rect.y, LANE_HEADER, rect.h), false, "fill=\"#f6f8fa\"");
            if index > 0 {
                line(svg, Pos::new(rect.x, rect.y), Pos::new(rect.right(), rect.y), "");
            }
            svg.push_str("</g>\n");
        }
    }
    svg.push_str("</g>\n");
}

fn draw_node(svg: &mut String, node: &Node) {
    let r = node.rect();
    let c = node.pos;
    match node.kind {
        NodeKind::Task(kind) => {
            rectangle(svg, r, true, "");
            task_marker(svg, Pos::new(r.x + 13.0, r.y + 12.0), kind);
        }
        NodeKind::SubProcess => {
            rectangle(svg, r, true, "");
            let center = Pos::new(c.x, r.bottom() - 9.0);
            rectangle(svg, Rect::from_center(center, 12.0, 12.0), false, "");
            line(svg, center - Pos::new(4.0, 0.0), center + Pos::new(4.0, 0.0), "");
            line(svg, center - Pos::new(0.0, 4.0), center + Pos::new(0.0, 4.0), "");
        }
        NodeKind::StartEvent(trigger) | NodeKind::EndEvent(trigger) | NodeKind::IntermediateEvent { trigger, .. } => {
            let end = matches!(node.kind, NodeKind::EndEvent(_));
            let intermediate = matches!(node.kind, NodeKind::IntermediateEvent { .. });
            circle(svg, c, r.w.min(r.h) / 2.0, if end { "stroke-width=\"3.5\"" } else { "" });
            if intermediate {
                circle(svg, c, r.w.min(r.h) / 2.0 - 3.0, "fill=\"none\" stroke-width=\"1\"");
            }
            let filled = end || matches!(node.kind, NodeKind::IntermediateEvent { throwing: true, .. });
            event_marker(svg, c, r.w.min(r.h) * 0.24, trigger, filled);
        }
        NodeKind::Gateway(gateway) => {
            writeln!(
                svg,
                "<polygon points=\"{},{} {},{} {},{} {},{}\"/>",
                c.x,
                r.y,
                r.right(),
                c.y,
                c.x,
                r.bottom(),
                r.x,
                c.y
            )
            .unwrap();
            let size = r.w.min(r.h) * 0.2;
            match gateway {
                GatewayType::Exclusive => {
                    line(svg, c - Pos::new(size, size), c + Pos::new(size, size), "stroke-width=\"3\"");
                    line(svg, c + Pos::new(-size, size), c + Pos::new(size, -size), "stroke-width=\"3\"");
                }
                GatewayType::Parallel => {
                    line(
                        svg,
                        c - Pos::new(size * 1.2, 0.0),
                        c + Pos::new(size * 1.2, 0.0),
                        "stroke-width=\"3\"",
                    );
                    line(
                        svg,
                        c - Pos::new(0.0, size * 1.2),
                        c + Pos::new(0.0, size * 1.2),
                        "stroke-width=\"3\"",
                    );
                }
                GatewayType::Inclusive => circle(svg, c, size * 1.1, "fill=\"none\" stroke-width=\"2.5\""),
                GatewayType::EventBased => {
                    circle(svg, c, size * 1.3, "fill=\"none\"");
                    circle(svg, c, size * 1.05, "fill=\"none\" stroke-width=\"1\"");
                    let points = (0..5)
                        .map(|i| {
                            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
                            format!("{},{}", c.x + size * 0.7 * angle.cos(), c.y + size * 0.7 * angle.sin())
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    writeln!(svg, "<polygon points=\"{points}\" fill=\"none\"/>").unwrap();
                }
                GatewayType::Complex => {
                    for i in 0..4 {
                        let angle = i as f32 * std::f32::consts::FRAC_PI_4;
                        let delta = Pos::new(angle.cos() * size, angle.sin() * size);
                        line(svg, c - delta, c + delta, "stroke-width=\"2.5\"");
                    }
                }
            }
        }
        NodeKind::DataObject => {
            let fold = r.w * 0.3;
            writeln!(
                svg,
                "<path d=\"M{} {}H{}L{} {}V{}H{}Z\"/><path d=\"M{} {}V{}H{}\" fill=\"none\"/>",
                r.x,
                r.y,
                r.right() - fold,
                r.right(),
                r.y + fold,
                r.bottom(),
                r.x,
                r.right() - fold,
                r.y,
                r.y + fold,
                r.right()
            )
            .unwrap();
        }
        NodeKind::DataStore => {
            let ry = r.h * 0.12;
            writeln!(
                svg,
                "<path d=\"M{} {}V{}A{} {} 0 0 0 {} {}V{}\"/><ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{ry}\"/>",
                r.x,
                r.y + ry,
                r.bottom() - ry,
                r.w / 2.0,
                ry,
                r.right(),
                r.bottom() - ry,
                r.y + ry,
                c.x,
                r.y + ry,
                r.w / 2.0
            )
            .unwrap();
        }
        NodeKind::TextAnnotation => {
            writeln!(
                svg,
                "<path d=\"M{} {}H{}V{}H{}\" fill=\"none\"/>",
                r.x + 10.0,
                r.y,
                r.x,
                r.bottom(),
                r.x + 10.0
            )
            .unwrap();
        }
    }
}

fn event_marker(svg: &mut String, c: Pos, size: f32, trigger: EventTrigger, filled: bool) {
    let fill = if filled { INK } else { "white" };
    match trigger {
        EventTrigger::None => {}
        EventTrigger::Message => {
            let rect = Rect::from_center(c, size * 1.6, size * 1.1);
            rectangle(svg, rect, false, &format!("fill=\"{fill}\" stroke-width=\"1\""));
            writeln!(
                svg,
                "<path d=\"M{} {}L{} {}L{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                rect.x,
                rect.y,
                c.x,
                c.y + size * 0.1,
                rect.right(),
                rect.y,
                if filled { "white" } else { INK }
            )
            .unwrap();
        }
        EventTrigger::Timer => {
            circle(svg, c, size * 0.9, "fill=\"none\" stroke-width=\"1\"");
            line(svg, c, c - Pos::new(0.0, size * 0.65), "stroke-width=\"1\"");
            line(svg, c, c + Pos::new(size * 0.55, 0.0), "stroke-width=\"1\"");
        }
        EventTrigger::Signal => {
            writeln!(
                svg,
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{fill}\"/>",
                c.x,
                c.y - size,
                c.x + size,
                c.y + size * 0.75,
                c.x - size,
                c.y + size * 0.75
            )
            .unwrap();
        }
        EventTrigger::Terminate => circle(svg, c, size, &format!("fill=\"{INK}\"")),
        EventTrigger::Conditional => {
            rectangle(svg, Rect::from_center(c, size * 1.3, size * 1.7), false, "stroke-width=\"1\"");
            for row in [-0.4, 0.0, 0.4] {
                line(
                    svg,
                    c + Pos::new(-size * 0.4, size * row),
                    c + Pos::new(size * 0.4, size * row),
                    "stroke-width=\"1\"",
                );
            }
        }
        EventTrigger::Error => {
            writeln!(
                svg,
                "<path d=\"M{} {}L{} {}L{} {}L{} {}L{} {}L{} {}Z\" fill=\"{fill}\"/>",
                c.x - size * 0.7,
                c.y + size * 0.8,
                c.x - size * 0.2,
                c.y - size * 0.9,
                c.x + size * 0.2,
                c.y - size * 0.1,
                c.x + size * 0.7,
                c.y - size * 0.8,
                c.x + size * 0.2,
                c.y + size * 0.9,
                c.x - size * 0.2,
                c.y + size * 0.1
            )
            .unwrap();
        }
    }
}

fn task_marker(svg: &mut String, c: Pos, kind: TaskType) {
    match kind {
        TaskType::Abstract => {}
        TaskType::Send | TaskType::Receive => event_marker(svg, c, 7.0, EventTrigger::Message, kind == TaskType::Send),
        TaskType::User => {
            circle(svg, c - Pos::new(0.0, 3.0), 2.5, "fill=\"none\" stroke-width=\"1\"");
            writeln!(
                svg,
                "<path d=\"M{} {}Q{} {} {} {}\" fill=\"none\" stroke-width=\"1\"/>",
                c.x - 5.0,
                c.y + 5.0,
                c.x,
                c.y - 3.0,
                c.x + 5.0,
                c.y + 5.0
            )
            .unwrap();
        }
        TaskType::Service => {
            let points = (0..16)
                .map(|index| {
                    let angle = index as f32 * std::f32::consts::TAU / 16.0;
                    let radius = if index % 2 == 0 { 6.0 } else { 4.0 };
                    format!("{},{}", c.x + radius * angle.cos(), c.y + radius * angle.sin())
                })
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(svg, "<polygon points=\"{points}\" fill=\"none\" stroke-width=\"1\"/>").unwrap();
            circle(svg, c, 2.0, "fill=\"none\" stroke-width=\"1\"");
        }
        TaskType::Script | TaskType::BusinessRule => {
            rectangle(svg, Rect::from_center(c, 12.0, 12.0), false, "stroke-width=\"1\"");
            for row in [-3.0, 1.0] {
                line(svg, c + Pos::new(-5.0, row), c + Pos::new(5.0, row), "stroke-width=\"1\"");
            }
            if kind == TaskType::BusinessRule {
                line(svg, c - Pos::new(2.0, 6.0), c + Pos::new(-2.0, 6.0), "stroke-width=\"1\"");
            }
        }
        TaskType::Manual => {
            writeln!(
                svg,
                "<path d=\"M{} {}l-4-4v-3h2v-4h2v4h2v-3h2v4h2v4l-3 4Z\" fill=\"none\" stroke-width=\"1\"/>",
                c.x - 2.0,
                c.y + 6.0
            )
            .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_user_content_is_escaped_and_the_model_is_unchanged() {
        let hostile = "Pizza & \"quote\" <script onload=\"alert(1)\">\nsecond line";
        let mut diagram = BpmnDiagram::new(hostile);
        let pool = diagram.add_pool(Pos::new(-100.0, -100.0));
        diagram.pool_mut(pool).unwrap().name = hostile.into();
        diagram.add_lane(pool);
        diagram.pool_mut(pool).unwrap().lanes[1].name = hostile.into();
        let first = diagram.add_node(NodeKind::Task(TaskType::User), Pos::new(100.0, 30.0));
        let second = diagram.add_node(NodeKind::Task(TaskType::Service), Pos::new(300.0, 30.0));
        diagram.node_mut(first).unwrap().name = hostile.into();
        diagram.node_mut(first).unwrap().documentation = hostile.into();
        let flow = diagram.add_flow(first, second).unwrap();
        diagram.flow_mut(flow).unwrap().name = hostile.into();
        let original = diagram.clone();
        let svg = diagram.to_svg();
        assert_eq!(diagram, original);
        assert!(!svg.contains("<script"));
        let root = crate::xml::parse(&svg).unwrap();
        assert_eq!(root.name, "svg");
        assert_eq!(root.text_at(&["title"]), Some(hostile));
        let groups: Vec<_> = root.descendants().into_iter().filter(|element| element.name == "g").collect();
        assert!(groups.iter().any(|element| element.attr("data-name") == Some(hostile)));
        assert!(
            root.descendants()
                .iter()
                .all(|element| element.attr("onload").is_none() && element.name != "script")
        );
        assert!(
            root.descendants()
                .iter()
                .any(|element| element.name == "desc" && element.text == hostile)
        );
    }

    #[test]
    fn whole_diagram_bounds_cover_backward_routes_and_wrapped_labels() {
        let mut diagram = BpmnDiagram::new("Looping process");
        let first = diagram.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(-180.0, -60.0));
        let last = diagram.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(350.0, 140.0));
        let artifact = diagram.add_node(NodeKind::DataObject, Pos::new(500.0, 170.0));
        diagram.node_mut(artifact).unwrap().name =
            "Long data object label with enough words to wrap across several lines below its shape".into();
        let back = diagram.add_flow(last, first).unwrap();
        diagram.flow_mut(back).unwrap().name = "Retry after an unsuccessful attempt".into();
        let root = crate::xml::parse(&diagram.to_svg()).unwrap();
        let values: Vec<f32> = root
            .attr("viewBox")
            .unwrap()
            .split_whitespace()
            .map(|value| value.parse().unwrap())
            .collect();
        let bounds = Rect::new(values[0], values[1], values[2], values[3]);
        for node in &diagram.nodes {
            assert!(bounds.contains(Pos::new(node.rect().x, node.rect().y)));
            assert!(bounds.contains(Pos::new(node.rect().right(), node.rect().bottom())));
        }
        for point in route(&diagram, diagram.flow(back).unwrap()) {
            assert!(bounds.contains(point));
        }
        let label = node_label(diagram.node(artifact).unwrap());
        assert!(label.lines.len() > 1);
        assert!(bounds.contains(Pos::new(label.bounds().right(), label.bounds().bottom())));
        assert!(root.descendants().iter().filter(|element| element.name == "tspan").count() > diagram.nodes.len());
    }

    #[test]
    fn bpmn_shapes_and_connection_semantics_are_exported_as_vectors() {
        let mut diagram = BpmnDiagram::new("BPMN glyphs");
        let tasks: Vec<_> = TaskType::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| diagram.add_node(NodeKind::Task(kind), Pos::new(100.0 + index as f32 * 150.0, 100.0)))
            .collect();
        let gateways: Vec<_> = GatewayType::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| diagram.add_node(NodeKind::Gateway(kind), Pos::new(100.0 + index as f32 * 150.0, 240.0)))
            .collect();
        for (index, trigger) in EventTrigger::ALL.into_iter().enumerate() {
            let x = 100.0 + index as f32 * 150.0;
            diagram.add_node(NodeKind::StartEvent(trigger), Pos::new(x, 340.0));
            diagram.add_node(NodeKind::IntermediateEvent { trigger, throwing: true }, Pos::new(x, 440.0));
            diagram.add_node(NodeKind::EndEvent(trigger), Pos::new(x, 540.0));
        }
        let object = diagram.add_node(NodeKind::DataObject, Pos::new(900.0, 240.0));
        diagram.add_node(NodeKind::DataStore, Pos::new(1050.0, 240.0));
        diagram.add_node(NodeKind::TextAnnotation, Pos::new(1200.0, 240.0));
        diagram.add_node(NodeKind::SubProcess, Pos::new(1200.0, 440.0));
        diagram.add_flow(tasks[0], object).unwrap();
        let message = diagram.add_flow(tasks[1], tasks[2]).unwrap();
        diagram.flow_mut(message).unwrap().kind = FlowKind::Message;
        let default = diagram.add_flow(gateways[0], tasks[3]).unwrap();
        diagram.flow_mut(default).unwrap().is_default = true;
        let svg = diagram.to_svg();
        let root = crate::xml::parse(&svg).unwrap();
        assert_eq!(
            root.descendants()
                .iter()
                .filter(|element| element.name == "g" && element.attr("id").is_some_and(|id| id.starts_with("node")))
                .count(),
            diagram.nodes.len()
        );
        assert!(svg.contains("marker-end=\"url(#bpmn-sequence)\""));
        assert!(svg.contains("marker-end=\"url(#bpmn-message)\""));
        assert!(svg.contains("marker-end=\"url(#bpmn-open)\""));
        assert!(root.descendants().iter().any(|element| element.name == "ellipse"));
        assert!(root.descendants().iter().any(|element| element.name == "polygon"));
        assert!(root.descendants().iter().any(|element| element.name == "circle"));
        assert!(BpmnDiagram::new("Empty").to_svg().contains("viewBox=\"-24 -24 688 528\""));
    }
}
