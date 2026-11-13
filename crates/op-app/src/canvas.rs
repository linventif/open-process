//! Shared canvas utilities: view transform (pan/zoom), grid, palette and drawing helpers.

use egui::epaint::{CubicBezierShape, PathStroke};
use egui::text::LayoutJob;
use egui::{Align, Color32, FontId, Painter, Pos2, Rect, Response, Shape, Stroke, Ui, Vec2, pos2, vec2};
use op_core::geom::Pos;
use op_core::i18n::tr;
use serde::{Deserialize, Serialize};

/// Pan & zoom of an infinite canvas.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct View {
    /// Screen offset (relative to the canvas top-left) of the world origin.
    pub offset: Vec2,
    pub zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            offset: vec2(40.0, 40.0),
            zoom: 1.0,
        }
    }
}

impl View {
    pub fn to_screen(self, canvas: Rect, p: Pos) -> Pos2 {
        canvas.min + self.offset + vec2(p.x, p.y) * self.zoom
    }

    pub fn to_world(self, canvas: Rect, s: Pos2) -> Pos {
        let v = (s - canvas.min - self.offset) / self.zoom;
        Pos::new(v.x, v.y)
    }

    pub fn rect_to_screen(&self, canvas: Rect, r: op_core::geom::Rect) -> Rect {
        Rect::from_min_size(self.to_screen(canvas, Pos::new(r.x, r.y)), vec2(r.w, r.h) * self.zoom)
    }

    /// Zoom by `factor` keeping the world point under `anchor` fixed.
    pub fn zoom_around(&mut self, canvas: Rect, anchor: Pos2, factor: f32) {
        let before = self.to_world(canvas, anchor);
        self.zoom = (self.zoom * factor).clamp(0.1, 5.0);
        let after = self.to_screen(canvas, before);
        self.offset += anchor - after;
    }

    /// Mouse wheel / pinch / middle- and right-drag navigation.
    pub fn handle_navigation(&mut self, ui: &Ui, response: &Response, canvas: Rect) {
        if response.hovered() {
            let (zoom, scroll, pointer) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta(), i.pointer.hover_pos()));
            if zoom != 1.0
                && let Some(p) = pointer
            {
                self.zoom_around(canvas, p, zoom);
            }
            if scroll != Vec2::ZERO {
                self.offset += scroll;
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) || response.dragged_by(egui::PointerButton::Secondary) {
            self.offset += response.drag_delta();
        }
    }

    /// Fits a world-space bounding box into the canvas.
    pub fn fit(&mut self, canvas: Rect, min: Pos, max: Pos) {
        let w = (max.x - min.x).max(1.0);
        let h = (max.y - min.y).max(1.0);
        let margin = 60.0;
        let zx = (canvas.width() - 2.0 * margin) / w;
        let zy = (canvas.height() - 2.0 * margin) / h;
        self.zoom = zx.min(zy).clamp(0.2, 1.5);
        let center = vec2((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
        self.offset = canvas.size() / 2.0 - center * self.zoom;
    }
}

/// Theme-aware colors used by the diagram renderers.
#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub grid: Color32,
    pub ink: Color32,
    pub arc: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub fill: Color32,
    pub task_fill: Color32,
    pub pool_header: Color32,
    pub border: Color32,
    pub select: Color32,
    pub ok: Color32,
    pub bad: Color32,
    pub warn: Color32,
    pub start: Color32,
    pub end: Color32,
    pub gateway: Color32,
    pub token: Color32,
    pub transition: Color32,
    pub silent: Color32,
    pub shadow: Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        let c = Color32::from_rgb;
        if dark {
            Palette {
                dark,
                bg: c(22, 24, 28),
                grid: c(46, 50, 58),
                ink: c(176, 184, 198),
                arc: c(140, 148, 164),
                text: c(228, 231, 236),
                muted: c(150, 156, 168),
                fill: c(34, 37, 44),
                task_fill: c(33, 42, 62),
                pool_header: c(38, 41, 48),
                border: c(54, 58, 66),
                select: crate::theme::ACCENT_DARK,
                ok: c(64, 200, 120),
                bad: c(248, 100, 100),
                warn: c(245, 176, 66),
                start: c(64, 200, 120),
                end: c(248, 100, 100),
                gateway: c(240, 190, 70),
                token: c(236, 238, 242),
                transition: c(196, 204, 218),
                silent: c(96, 104, 118),
                shadow: Color32::from_black_alpha(90),
            }
        } else {
            Palette {
                dark,
                bg: c(252, 252, 253),
                grid: c(212, 216, 224),
                ink: c(58, 64, 78),
                arc: c(96, 104, 120),
                text: c(30, 34, 40),
                muted: c(110, 117, 130),
                fill: Color32::WHITE,
                task_fill: c(240, 245, 255),
                pool_header: c(244, 245, 248),
                border: c(225, 228, 234),
                select: crate::theme::ACCENT,
                ok: c(22, 163, 74),
                bad: c(220, 38, 38),
                warn: c(217, 119, 6),
                start: c(22, 163, 74),
                end: c(220, 38, 38),
                gateway: c(214, 158, 20),
                token: c(32, 36, 44),
                transition: c(44, 50, 64),
                silent: c(160, 166, 178),
                shadow: Color32::from_black_alpha(22),
            }
        }
    }

    pub fn for_ui(ui: &Ui) -> Self {
        Self::new(ui.visuals().dark_mode)
    }
}

/// Soft drop shadow under a rounded rectangle.
pub fn shadow_rect(painter: &Painter, rect: Rect, radius: f32, pal: &Palette) {
    for (k, a) in [(1.0f32, 1.0f32), (2.5, 0.55), (4.5, 0.25)] {
        painter.rect_filled(
            rect.translate(vec2(0.0, k * 0.8)).expand(k * 0.6),
            radius + k,
            pal.shadow.gamma_multiply(a * 0.6),
        );
    }
}

/// Soft drop shadow under a circle.
pub fn shadow_circle(painter: &Painter, c: Pos2, r: f32, pal: &Palette) {
    for (k, a) in [(1.0f32, 1.0f32), (2.5, 0.55), (4.5, 0.25)] {
        painter.circle_filled(c + vec2(0.0, k * 0.8), r + k * 0.6, pal.shadow.gamma_multiply(a * 0.6));
    }
}

pub fn draw_background(painter: &Painter, canvas: Rect, view: &View, pal: &Palette) {
    painter.rect_filled(canvas, 0.0, pal.bg);
    let step = 20.0 * view.zoom;
    if step < 6.0 {
        return;
    }
    let origin = canvas.min + view.offset;
    let start_x = origin.x - ((origin.x - canvas.min.x) / step).floor() * step;
    let start_y = origin.y - ((origin.y - canvas.min.y) / step).floor() * step;
    let r = (view.zoom * 1.1).clamp(0.6, 1.5);
    let mut y = start_y;
    while y < canvas.max.y {
        let mut x = start_x;
        while x < canvas.max.x {
            painter.circle_filled(pos2(x, y), r, pal.grid);
            x += step;
        }
        y += step;
    }
}

/// Filled (or open) triangular arrow head with its tip at `tip`.
pub fn arrow_head(painter: &Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32, filled: bool) {
    let d = dir.normalized();
    if !d.is_finite() {
        return;
    }
    let n = vec2(-d.y, d.x);
    let base = tip - d * size;
    let a = base + n * size * 0.45;
    let b = base - n * size * 0.45;
    if filled {
        painter.add(Shape::convex_polygon(vec![tip, a, b], color, Stroke::NONE));
    } else {
        painter.add(Shape::line(vec![a, tip, b], Stroke::new(1.5, color)));
    }
}

pub fn polyline(painter: &Painter, pts: &[Pos2], stroke: Stroke) {
    if pts.len() >= 2 {
        painter.add(Shape::line(pts.to_vec(), PathStroke::from(stroke)));
    }
}

pub fn dashed(painter: &Painter, pts: &[Pos2], stroke: Stroke, dash: f32, gap: f32) {
    painter.extend(Shape::dashed_line(pts, stroke, dash, gap));
}

pub fn bezier(painter: &Painter, pts: [Pos2; 4], stroke: Stroke) {
    painter.add(CubicBezierShape::from_points_stroke(pts, false, Color32::TRANSPARENT, stroke));
}

thread_local! {
    static HAND_LABELS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Diagram labels use the handwriting font while sketch mode is being drawn.
pub fn set_hand_labels(on: bool) {
    HAND_LABELS.with(|c| c.set(on));
}

fn label_font(size: f32) -> FontId {
    if HAND_LABELS.with(|c| c.get()) {
        crate::theme::hand(size * 1.15)
    } else {
        FontId::proportional(size)
    }
}

/// Draws `text` wrapped to `width` and centered on `center`. Returns its screen rect.
pub fn text_centered(painter: &Painter, center: Pos2, width: f32, text: &str, size: f32, color: Color32) -> Rect {
    if text.is_empty() {
        return Rect::from_center_size(center, Vec2::ZERO);
    }
    let mut job = LayoutJob::simple(text.to_owned(), label_font(size), color, width.max(10.0));
    job.halign = Align::Center;
    let galley = painter.layout_job(job);
    let pos = center - galley.rect.center().to_vec2();
    let rect = galley.rect.translate(pos.to_vec2());
    painter.galley(pos, galley, color);
    rect
}

/// Same as [`text_centered`] but anchored by the top-center.
/// With `bg`, a translucent background keeps the label readable over arcs.
pub fn text_below(painter: &Painter, top_center: Pos2, width: f32, text: &str, size: f32, color: Color32, bg: Option<Color32>) {
    if text.is_empty() {
        return;
    }
    let mut job = LayoutJob::simple(text.to_owned(), label_font(size), color, width.max(10.0));
    job.halign = Align::Center;
    let galley = painter.layout_job(job);
    let pos = pos2(top_center.x - galley.rect.center().x, top_center.y - galley.rect.min.y);
    if let Some(bg) = bg {
        painter.rect_filled(galley.rect.translate(pos.to_vec2()).expand(1.5), 3.0, bg.gamma_multiply(0.85));
    }
    painter.galley(pos, galley, color);
}

/// Small text with a background "halo" so labels stay readable over lines.
pub fn label_with_halo(painter: &Painter, pos: Pos2, text: &str, size: f32, pal: &Palette) {
    if text.is_empty() {
        return;
    }
    let galley = painter.layout_no_wrap(text.to_owned(), label_font(size), pal.text);
    let rect = Rect::from_center_size(pos, galley.size()).expand(2.0);
    painter.rect_filled(rect, 3.0, pal.bg.gamma_multiply(0.85));
    painter.galley(rect.min + vec2(2.0, 2.0), galley, pal.text);
}

pub fn to_pos2(p: Pos) -> Pos2 {
    pos2(p.x, p.y)
}

/// Point at the middle (by length) of a polyline.
pub fn polyline_midpoint(pts: &[Pos2]) -> Pos2 {
    if pts.is_empty() {
        return Pos2::ZERO;
    }
    let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if acc + l >= total / 2.0 && l > 0.0 {
            return w[0].lerp(w[1], (total / 2.0 - acc) / l);
        }
        acc += l;
    }
    pts[0]
}

/// A square tool button showing a custom-painted icon.
pub fn icon_button(ui: &mut Ui, selected: bool, tooltip: &str, paint: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let size = vec2(36.0, 36.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let accent = crate::theme::accent(ui.visuals().dark_mode);
    let enabled = ui.is_enabled();
    if selected {
        ui.painter().rect_filled(rect, 8.0, accent);
    } else if response.hovered() && enabled {
        ui.painter().rect_filled(rect, 8.0, ui.visuals().widgets.hovered.bg_fill);
    }
    let color = if selected {
        Color32::WHITE
    } else if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    paint(ui.painter(), rect.shrink(9.0), color);
    let name = tooltip.lines().next().unwrap_or(tooltip).to_owned();
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, selected, name.clone()));
    response.on_hover_text(tooltip)
}

/// Horizontal rule between groups of a vertical toolbar.
pub fn toolbar_separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(28.0, 9.0), egui::Sense::hover());
    ui.painter().line_segment(
        [pos2(rect.left(), rect.center().y), pos2(rect.right(), rect.center().y)],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

/// Floating, rounded panel drawn over a canvas (tool palettes, banners).
pub fn floating(ctx: &egui::Context, id: egui::Id, pos: Pos2, pivot: egui::Align2, add: impl FnOnce(&mut Ui)) -> Rect {
    egui::Area::new(id)
        .fixed_pos(pos)
        .pivot(pivot)
        .default_height(2000.0)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let v = ui.visuals();
            egui::Frame::new()
                .fill(v.window_fill)
                .stroke(Stroke::new(1.0, v.widgets.noninteractive.bg_stroke.color))
                .corner_radius(12.0)
                .shadow(v.popup_shadow)
                .inner_margin(6.0)
                .show(ui, add);
        })
        .response
        .rect
}

/// Slim, sharp arrow head.
pub fn arrow_head_slim(painter: &Painter, tip: Pos2, dir: Vec2, size: f32, color: Color32) {
    let d = dir.normalized();
    if !d.is_finite() {
        return;
    }
    let n = vec2(-d.y, d.x);
    let base = tip - d * size;
    let notch = tip - d * size * 0.78;
    painter.add(Shape::convex_polygon(
        vec![tip, base + n * size * 0.38, notch, base - n * size * 0.38],
        color,
        Stroke::NONE,
    ));
}

/// Point at fraction `t` (0..1) of the length of a polyline.
pub fn point_along(pts: &[Pos2], t: f32) -> Pos2 {
    if pts.len() < 2 {
        return pts.first().copied().unwrap_or(Pos2::ZERO);
    }
    let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let target = total * t.clamp(0.0, 1.0);
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if acc + l >= target && l > 0.0 {
            return w[0].lerp(w[1], (target - acc) / l);
        }
        acc += l;
    }
    *pts.last().unwrap()
}

/// Small rounded label ("pill") used for arc weights, capacities…
pub fn badge(painter: &Painter, center: Pos2, text: &str, size: f32, color: Color32, pal: &Palette) {
    let galley = painter.layout_no_wrap(text.to_owned(), crate::theme::semibold(size), color);
    let s = galley.size();
    let rect = Rect::from_center_size(center, vec2((s.x + size * 0.9).max(s.y + 2.0), s.y + 2.0));
    painter.rect(
        rect,
        rect.height() / 2.0,
        pal.fill,
        Stroke::new(1.0, color.gamma_multiply(0.6)),
        egui::StrokeKind::Middle,
    );
    painter.galley(rect.center() - s / 2.0, galley, color);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InlineOutcome {
    Editing,
    Done,
    Cancelled,
}

/// A floating single-line text editor placed on the canvas (double-click to rename).
pub fn inline_editor(ctx: &egui::Context, id: egui::Id, pos: Pos2, width: f32, text: &mut String, first_frame: bool) -> InlineOutcome {
    let mut outcome = InlineOutcome::Editing;
    egui::Area::new(id).fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            let r = ui.add(egui::TextEdit::singleline(text).desired_width(width.max(80.0)));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, tr("Name")));
            if first_frame {
                r.request_focus();
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                outcome = InlineOutcome::Cancelled;
            } else if r.lost_focus() {
                outcome = InlineOutcome::Done;
            }
        });
    });
    outcome
}

/// ✓ / ✗ / ? indicator for a property that may be undecided.
pub fn property_row(ui: &mut Ui, label: &str, value: Option<bool>, hint: &str) {
    let pal = Palette::for_ui(ui);
    let (sym, color) = match value {
        Some(true) => ("✔", pal.ok),
        Some(false) => ("✖", pal.bad),
        None => ("?", pal.warn),
    };
    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 6.0, color.gamma_multiply(if pal.dark { 0.25 } else { 0.13 }));
    ui.painter()
        .text(rect.center(), egui::Align2::CENTER_CENTER, sym, crate::theme::semibold(11.0), color);
    let r = ui.label(label);
    if !hint.is_empty() {
        r.on_hover_text(hint);
    }
    ui.end_row();
}

/// Status bar zoom control: − 100% + and "fit".
pub fn zoom_controls(ui: &mut Ui, view: &mut View, canvas: Rect, fit: &mut bool) {
    ui.spacing_mut().item_spacing.x = 2.0;
    if ui.small_button(tr("Fit")).on_hover_text(tr("Zoom to fit (F)")).clicked() {
        *fit = true;
    }
    ui.add_space(6.0);
    if ui.small_button("+").on_hover_text(tr("Zoom in")).clicked() {
        view.zoom_around(canvas, canvas.center(), 1.2);
    }
    let pct = egui::RichText::new(format!("{:>3.0}%", view.zoom * 100.0)).monospace();
    if ui
        .add(egui::Button::new(pct).frame(false))
        .on_hover_text(tr("Reset zoom to 100%"))
        .clicked()
    {
        view.zoom_around(canvas, canvas.center(), 1.0 / view.zoom);
    }
    if ui.small_button("−").on_hover_text(tr("Zoom out")).clicked() {
        view.zoom_around(canvas, canvas.center(), 1.0 / 1.2);
    }
}

/// Tiny pseudo-random generator (xorshift) for the token game.
#[derive(Clone, Copy, Debug)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0.max(1);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Process-wide unique id for editor instances (keeps floating widgets of
/// side-by-side editors apart).
pub fn next_uid() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Sketch ("hand-drawn") rendering, in the spirit of Excalidraw / rough.js.
// Randomness is seeded per element so drawings do not flicker.

fn jit(rng: &mut Rng, amp: f32) -> f32 {
    ((rng.next() % 2001) as f32 / 1000.0 - 1.0) * amp
}

/// A rough line through `pts` (closed if `closed`), drawn twice with small offsets.
pub fn sketch_polyline(painter: &Painter, pts: &[Pos2], stroke: Stroke, seed: u64, closed: bool) {
    if pts.len() < 2 {
        return;
    }
    let mut segs: Vec<(Pos2, Pos2)> = pts.windows(2).map(|w| (w[0], w[1])).collect();
    if closed {
        segs.push((pts[pts.len() - 1], pts[0]));
    }
    for pass in 0..2u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9).wrapping_add(pass * 7919 + 1));
        for &(a, b) in &segs {
            let len = a.distance(b);
            if len < 0.5 {
                continue;
            }
            let amp = (len * 0.012).clamp(0.3, 1.8);
            let d = (b - a) / len;
            let n = vec2(-d.y, d.x);
            let a2 = a + vec2(jit(&mut rng, amp), jit(&mut rng, amp));
            let b2 = b + vec2(jit(&mut rng, amp), jit(&mut rng, amp));
            let mid = a.lerp(b, 0.5 + jit(&mut rng, 0.08)) + n * jit(&mut rng, (len * 0.02).min(3.0));
            let line: Vec<Pos2> = (0..=6)
                .map(|i| {
                    let t = i as f32 / 6.0;
                    let u = 1.0 - t;
                    pos2(
                        u * u * a2.x + 2.0 * u * t * mid.x + t * t * b2.x,
                        u * u * a2.y + 2.0 * u * t * mid.y + t * t * b2.y,
                    )
                })
                .collect();
            let w = stroke.width * if pass == 0 { 1.0 } else { 0.75 };
            painter.add(Shape::line(line, Stroke::new(w, stroke.color)));
        }
    }
}

/// A rough ellipse/circle, slightly overshooting its start point.
pub fn sketch_ellipse(painter: &Painter, c: Pos2, r: Vec2, stroke: Stroke, seed: u64) {
    for pass in 0..2u64 {
        let mut rng = Rng(seed.wrapping_mul(0x85EB_CA6B).wrapping_add(pass * 104_729 + 3));
        let start = jit(&mut rng, std::f32::consts::PI);
        let wobble = 1.0 + jit(&mut rng, 0.04);
        let n = 30;
        let overshoot = 0.25 + jit(&mut rng, 0.1).abs();
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let t = i as f32 / n as f32;
                let a = start + t * (std::f32::consts::TAU + overshoot);
                let k = wobble * (1.0 + 0.025 * (a * 3.0 + start).sin());
                c + vec2(a.cos() * r.x * k, a.sin() * r.y * k)
            })
            .collect();
        let w = stroke.width * if pass == 0 { 1.0 } else { 0.7 };
        painter.add(Shape::line(pts, Stroke::new(w, stroke.color)));
    }
}

/// Rough rectangle outline.
pub fn sketch_rect(painter: &Painter, r: Rect, stroke: Stroke, seed: u64) {
    sketch_polyline(
        painter,
        &[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()],
        stroke,
        seed,
        true,
    );
}

/// Diagonal hachure fill of a rectangle.
pub fn hachure_rect(painter: &Painter, r: Rect, color: Color32, gap: f32, seed: u64) {
    let mut rng = Rng(seed ^ 0xA5A5);
    let (min, max) = (r.min, r.max);
    let mut k = min.x - max.y;
    while k < max.x - min.y {
        // line x = k + t, y = t  (45°)
        let t0 = (min.x - k).max(min.y);
        let t1 = (max.x - k).min(max.y);
        if t1 > t0 {
            let a = pos2(k + t0, t0) + vec2(jit(&mut rng, 0.6), jit(&mut rng, 0.6));
            let b = pos2(k + t1, t1) + vec2(jit(&mut rng, 0.6), jit(&mut rng, 0.6));
            painter.line_segment([a, b], Stroke::new(1.0, color));
        }
        k += gap;
    }
}

/// Rough open arrow head (two strokes).
pub fn sketch_arrow_head(painter: &Painter, tip: Pos2, dir: Vec2, size: f32, stroke: Stroke, seed: u64) {
    let d = dir.normalized();
    if !d.is_finite() {
        return;
    }
    let n = vec2(-d.y, d.x);
    let base = tip - d * size;
    sketch_polyline(
        painter,
        &[base + n * size * 0.5, tip, base - n * size * 0.5],
        stroke,
        seed ^ 0x51,
        false,
    );
}

thread_local! {
    static SKETCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Enables sketch rendering (and handwriting labels) for the diagram being drawn.
pub fn set_sketch(on: bool) {
    SKETCH.with(|c| c.set(on));
    set_hand_labels(on);
}

pub fn sketch() -> bool {
    SKETCH.with(|c| c.get())
}

/// Status-bar menu to choose the link style and the sketch mode.
pub fn style_menu(ui: &mut Ui, style: &mut op_core::style::DiagramStyle, petri: bool) {
    ui.menu_button(tr("Style ▾"), |ui| {
        if petri {
            use op_core::style::TransitionShape;
            crate::theme::section_title(ui, tr("Transitions"));
            ui.radio_value(&mut style.transitions, TransitionShape::Box, tr("Rectangle, name inside (course)"));
            ui.radio_value(&mut style.transitions, TransitionShape::Bar, tr("Thin bar, name below"));
        }
        crate::theme::section_title(ui, tr("Links"));
        for e in op_core::style::EdgeStyle::ALL {
            ui.radio_value(&mut style.edges, e, e.label());
        }
        crate::theme::section_title(ui, tr("Drawing"));
        ui.checkbox(&mut style.sketch, tr("Hand-drawn (sketch)"));
    })
    .response
    .on_hover_text(tr("Link style (straight, 90°, curved) and hand-drawn mode"));
}
