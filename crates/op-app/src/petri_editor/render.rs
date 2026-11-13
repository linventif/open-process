//! Render for the PetriEditor notation.

use super::*;

impl PetriEditor {
    pub(super) fn draw_net(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>) {
        let sketch = self.net.style.sketch;
        canvas::set_sketch(sketch);
        self.draw_net_inner(painter, canvas, pal, wp, sketch);
        canvas::set_sketch(false);
    }

    pub(super) fn draw_net_inner(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>, sketch: bool) {
        let v = &self.view;
        let z = v.zoom;
        let compiled = self.net.compile();
        let sim_on = self.sim.is_some();

        // Animation state of the last firing
        let anim = self
            .sim
            .as_ref()
            .and_then(|s| s.anim.clone())
            .filter(|a| self.now - a.start < a.duration as f64);
        let progress = anim
            .as_ref()
            .map(|a| (((self.now - a.start) / a.duration as f64) as f32).clamp(0.0, 1.0));
        let marking = match (&anim, &self.sim) {
            // during the animation, consumed tokens have left and produced ones have not arrived yet
            (Some(a), _) => {
                let mut m = a.before.clone();
                for &(p, w) in &compiled.inputs[a.t] {
                    m[p] = m[p].saturating_sub(w);
                }
                m
            }
            _ => self.current_marking(),
        };
        let enabled: Vec<bool> = match &self.sim {
            Some(s) => (0..self.net.transitions.len())
                .map(|t| compiled.is_enabled(t, &s.marking))
                .collect(),
            None => vec![false; self.net.transitions.len()],
        };
        let hover = wp.and_then(|w| self.hit(w));
        let focus = match (hover, self.selection) {
            (Some(Sel::Node(n)), _) => Some(n),
            (_, Some(Sel::Node(n))) => Some(n),
            _ => None,
        };
        let label_size = (12.0 * z).clamp(7.0, 22.0);

        // ---- arcs
        for a in &self.net.arcs {
            let Some(path) = self.arc_path(a.from, a.to) else { continue };
            let pts: Vec<Pos2> = path.iter().map(|&p| v.to_screen(canvas, p)).collect();
            let selected = self.selection == Some(Sel::Arc(a.id));
            let hovered = hover == Some(Sel::Arc(a.id));
            let related = focus.is_some_and(|n| a.from == n || a.to == n);
            let firing = anim.as_ref().is_some_and(|f| {
                a.from == NodeRef::Transition(self.net.transitions[f.t].id) || a.to == NodeRef::Transition(self.net.transitions[f.t].id)
            });
            let color = if selected || hovered || related {
                pal.select
            } else if firing {
                pal.ok
            } else {
                pal.arc
            };
            let width = if selected {
                2.4
            } else if hovered || related {
                1.9
            } else {
                1.4
            } * z.clamp(0.6, 1.6);
            let n = pts.len();
            let dir = pts[n - 1] - pts[n - 2];
            let head = 10.0 * z.clamp(0.5, 1.6);
            let mut body = pts.clone();
            match a.kind {
                ArcKind::Normal if sketch => {
                    canvas::sketch_polyline(painter, &body, Stroke::new(width, color), a.id as u64, false);
                    canvas::sketch_arrow_head(painter, pts[n - 1], dir, head, Stroke::new(width, color), a.id as u64);
                }
                ArcKind::Normal => {
                    body[n - 1] = pts[n - 1] - dir.normalized() * head * 0.75;
                    canvas::polyline(painter, &body, Stroke::new(width, color));
                    canvas::arrow_head_slim(painter, pts[n - 1], dir, head, color);
                }
                ArcKind::Inhibitor => {
                    let r = 4.5 * z.clamp(0.5, 1.6);
                    body[n - 1] = pts[n - 1] - dir.normalized() * 2.0 * r;
                    if sketch {
                        canvas::sketch_polyline(painter, &body, Stroke::new(width, color), a.id as u64, false);
                    } else {
                        canvas::polyline(painter, &body, Stroke::new(width, color));
                    }
                    painter.circle(pts[n - 1] - dir.normalized() * r, r, pal.fill, Stroke::new(width, color));
                }
            }
            if a.weight != 1 {
                let mid = canvas::point_along(&pts, 0.5);
                let nrm = vec2(-dir.y, dir.x).normalized() * 11.0 * z;
                canvas::badge(painter, mid + nrm, &a.weight.to_string(), (11.0 * z).clamp(8.0, 18.0), color, pal);
            }
        }

        // ---- places
        for (i, p) in self.net.places.iter().enumerate() {
            let c = v.to_screen(canvas, p.pos);
            let r = PLACE_R * z;
            let node = NodeRef::Place(p.id);
            let sel = self.in_selection(node);
            let hov = hover == Some(Sel::Node(node));
            if !sketch {
                canvas::shadow_circle(painter, c, r, pal);
            }
            if sel {
                painter.circle_stroke(c, r + 4.0 * z, Stroke::new(4.0 * z, pal.select.gamma_multiply(0.25)));
            }
            let stroke_c = if sel || hov { pal.select } else { pal.ink };
            let stroke = Stroke::new(if sel { 2.2 } else { 1.6 } * z.clamp(0.6, 1.6), stroke_c);
            if sketch {
                painter.circle_filled(c, r, pal.fill);
                canvas::sketch_ellipse(painter, c, vec2(r, r), stroke, p.id as u64);
            } else {
                painter.circle(c, r, pal.fill, stroke);
            }
            draw_tokens(painter, c, z * 0.5, marking[i], pal.token);
            canvas::text_below(
                painter,
                c + vec2(0.0, r + 4.0 * z),
                150.0 * z,
                &p.name,
                label_size,
                pal.text,
                Some(pal.bg),
            );
            if let Some(cap) = p.capacity {
                canvas::badge(
                    painter,
                    c + vec2(r * 0.85, -r * 0.85),
                    &format!("≤{cap}"),
                    label_size * 0.8,
                    pal.muted,
                    pal,
                );
            }
        }

        // ---- transitions
        for (i, t) in self.net.transitions.iter().enumerate() {
            let c = v.to_screen(canvas, t.pos);
            let boxed = self.net.style.transitions == op_core::style::TransitionShape::Box;
            let (tw, th) = self.trans_size(&t.name);
            let rect = Rect::from_center_size(c, vec2(tw, th) * z);
            let node = NodeRef::Transition(t.id);
            let sel = self.in_selection(node);
            let hov = hover == Some(Sel::Node(node));
            let is_firing = anim.as_ref().is_some_and(|a| a.t == i);
            let radius = (3.0 * z).min(255.0) as u8;
            if sim_on && (enabled[i] || is_firing) {
                let glow = if is_firing { pal.select } else { pal.ok };
                for k in [9.0f32, 6.0, 3.0] {
                    painter.rect_filled(
                        rect.expand(k * z),
                        CornerRadius::same(radius + (k * z) as u8),
                        glow.gamma_multiply(0.07 + if hov { 0.05 } else { 0.0 }),
                    );
                }
            } else if !sketch {
                canvas::shadow_rect(painter, rect, radius as f32, pal);
            }
            let fill = if is_firing {
                pal.select
            } else if sim_on && enabled[i] {
                pal.ok
            } else if t.silent {
                pal.silent
            } else {
                pal.transition
            };
            if boxed {
                // Course notation: rectangle with the name inside.
                let (bg, border) = if is_firing {
                    (pal.select.gamma_multiply(0.25), pal.select)
                } else if sim_on && enabled[i] {
                    (pal.ok.gamma_multiply(0.22), pal.ok)
                } else if t.silent {
                    (pal.silent.gamma_multiply(0.35), pal.ink)
                } else {
                    (pal.task_fill, pal.ink)
                };
                let stroke = Stroke::new(1.5 * z.clamp(0.6, 1.6), border);
                if sketch {
                    painter.rect_filled(rect, 0.0, pal.fill);
                    painter.rect_filled(rect, 0.0, bg);
                    canvas::sketch_rect(painter, rect, stroke, t.id as u64);
                } else {
                    painter.rect(rect, CornerRadius::same(radius), bg, stroke, StrokeKind::Middle);
                }
                let label = if t.name.is_empty() && t.silent { "τ" } else { t.name.as_str() };
                canvas::text_centered(painter, c, rect.width() + 40.0 * z, label, label_size, pal.text);
            } else if sketch {
                painter.rect_filled(rect, 0.0, pal.fill);
                canvas::hachure_rect(painter, rect, fill, (3.5 * z).max(2.5), t.id as u64);
                canvas::sketch_rect(painter, rect, Stroke::new(1.5 * z.clamp(0.6, 1.6), fill), t.id as u64);
            } else {
                painter.rect_filled(rect, CornerRadius::same(radius), fill);
            }
            if sel || hov {
                painter.rect_stroke(
                    rect.expand(3.0 * z),
                    CornerRadius::same(radius + 3),
                    Stroke::new(2.0, pal.select),
                    StrokeKind::Outside,
                );
            }
            if !boxed {
                canvas::text_below(
                    painter,
                    pos2(c.x, rect.max.y + 4.0 * z),
                    150.0 * z,
                    &t.name,
                    label_size,
                    pal.text,
                    Some(pal.bg),
                );
            }
        }

        // ---- travelling tokens
        if let (Some(a), Some(p)) = (&anim, progress) {
            let tid = NodeRef::Transition(self.net.transitions[a.t].id);
            let ease = |x: f32| {
                let x = x.clamp(0.0, 1.0);
                x * x * (3.0 - 2.0 * x)
            };
            let (phase_in, phase_out) = (ease(p * 2.0), ease(p * 2.0 - 1.0));
            for arc in &self.net.arcs {
                let (incoming, frac) = if arc.to == tid && arc.kind == ArcKind::Normal {
                    (true, phase_in)
                } else if arc.from == tid {
                    (false, phase_out)
                } else {
                    continue;
                };
                if (incoming && p >= 0.5) || (!incoming && p < 0.5) {
                    continue;
                }
                let Some(path) = self.arc_path(arc.from, arc.to) else { continue };
                let pts: Vec<Pos2> = path.iter().map(|&q| v.to_screen(canvas, q)).collect();
                let pos = canvas::point_along(&pts, frac);
                let r = 4.2 * z.clamp(0.6, 1.6);
                painter.circle_filled(pos, r + 3.0, pal.select.gamma_multiply(0.18));
                painter.circle_filled(pos, r, pal.token);
                if arc.weight > 1 {
                    canvas::badge(painter, pos + vec2(9.0, -9.0) * z, &arc.weight.to_string(), 9.0, pal.select, pal);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Side panel
}
