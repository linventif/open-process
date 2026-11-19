//! Whole-diagram PNG export. A screenshot only holds what the canvas shows, so
//! the canvas visits the diagram tile by tile at a fixed scale; each tile is
//! captured and copied into one image, then the original view is restored.
use super::View;
use egui::{Color32, ColorImage, Rect};
use op_core::geom::Pos;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Output pixels per world unit, before the size limits below.
const SCALE: f32 = 2.0;
const MAX_SIDE: f32 = 8192.0;
const MAX_PIXELS: f32 = 40_000_000.0;
/// World units kept around the diagram's bounds, e.g. for labels below shapes.
const MARGIN: f32 = 40.0;

static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// Whether any export is moving a canvas; previews must not capture it.
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed) > 0
}

type Slot = Arc<Mutex<Option<Arc<ColorImage>>>>;

pub struct ImageExport {
    name: String,
    restore: View,
    /// World point at the image's top-left corner.
    origin: Pos,
    /// Output physical pixels per world unit.
    scale: f32,
    size: [usize; 2],
    pixels: Vec<Color32>,
    grid: Option<Grid>,
    next: usize,
    pending: Option<Slot>,
}

/// Fixed when the first frame knows the canvas size.
struct Grid {
    ppp: f32,
    canvas: [usize; 2],
    tiles: Vec<[usize; 2]>,
}

impl ImageExport {
    /// `bounds` are the diagram's world extents; the visible area is used when
    /// the diagram is empty.
    pub fn start(bounds: Option<(Pos, Pos)>, view: View, canvas: Rect, mut name: String) -> Self {
        if !name.to_lowercase().ends_with(".png") {
            name.push_str(".png");
        }
        let (min, max) = bounds.unwrap_or_else(|| (view.to_world(canvas, canvas.min), view.to_world(canvas, canvas.max)));
        let origin = Pos::new(min.x - MARGIN, min.y - MARGIN);
        let (w, h) = ((max.x - min.x + 2.0 * MARGIN).max(1.0), (max.y - min.y + 2.0 * MARGIN).max(1.0));
        let scale = SCALE.min(MAX_SIDE / w).min(MAX_SIDE / h).min((MAX_PIXELS / (w * h)).sqrt());
        let size = [((w * scale).ceil() as usize).max(1), ((h * scale).ceil() as usize).max(1)];
        ACTIVE.fetch_add(1, Ordering::Relaxed);
        Self {
            name,
            restore: view,
            origin,
            scale,
            size,
            pixels: vec![Color32::TRANSPARENT; size[0] * size[1]],
            grid: None,
            next: 0,
            pending: None,
        }
    }

    /// Call before drawing. Collects a finished capture, then points `view` at
    /// the next tile. Returns `false` once done: the view is restored and the
    /// image is handed to [`crate::io::poll_png_export`] for saving.
    pub fn before_draw(&mut self, view: &mut View, canvas: Rect, ctx: &egui::Context) -> bool {
        let ppp = ctx.pixels_per_point();
        let canvas_px = [(canvas.width() * ppp).floor() as usize, (canvas.height() * ppp).floor() as usize];
        let grid = self.grid.get_or_insert_with(|| Grid {
            ppp,
            canvas: canvas_px,
            tiles: tiles(self.size, canvas_px),
        });
        if grid.ppp != ppp || grid.canvas != canvas_px || canvas_px.contains(&0) {
            *view = self.restore;
            crate::io::finish_png_export(Err(op_core::i18n::tr("file.image_export_resized").into()));
            return false;
        }
        let shot = self.pending.as_ref().and_then(|slot| slot.lock().unwrap().take());
        if let Some(shot) = shot {
            self.pending = None;
            let at = self.grid.as_ref().unwrap().tiles[self.next];
            self.copy(&shot, canvas, ppp, at);
            self.next += 1;
        }
        let grid = self.grid.as_ref().unwrap();
        if self.next == grid.tiles.len() {
            *view = self.restore;
            let result = crate::io::encode_rgba(self.size, &self.pixels).map(|png| (png, self.name.clone()));
            crate::io::finish_png_export(result);
            return false;
        }
        let [x, y] = grid.tiles[self.next];
        let zoom = self.scale / ppp;
        let world = Pos::new(self.origin.x + x as f32 / self.scale, self.origin.y + y as f32 / self.scale);
        view.zoom = zoom;
        view.offset = egui::vec2(-world.x, -world.y) * zoom;
        true
    }

    /// Call after drawing the tile without selection, hover or floating tools.
    pub fn after_draw(&mut self, ctx: &egui::Context) {
        if self.pending.is_none() {
            let slot: Slot = Arc::default();
            let target = slot.clone();
            let repaint = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |image| {
                *target.lock().unwrap() = Some(image);
                repaint.request_repaint();
            })));
            self.pending = Some(slot);
        }
        ctx.request_repaint();
    }

    fn copy(&mut self, shot: &ColorImage, canvas: Rect, ppp: f32, at: [usize; 2]) {
        let tile = shot.region(&canvas, Some(ppp));
        let width = tile.size[0].min(self.size[0].saturating_sub(at[0]));
        let height = tile.size[1].min(self.size[1].saturating_sub(at[1]));
        for row in 0..height {
            let from = row * tile.size[0];
            let to = (at[1] + row) * self.size[0] + at[0];
            self.pixels[to..to + width].copy_from_slice(&tile.pixels[from..from + width]);
        }
    }
}

impl Drop for ImageExport {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Per-frame driver for editors, called right before drawing: positions `view`
/// on the current tile. Returns whether this frame renders an export tile, to
/// be drawn without selection, hover or floating tools.
pub fn frame(export: &mut Option<ImageExport>, view: &mut View, canvas: Rect, ctx: &egui::Context) -> bool {
    let busy = export.as_mut().is_some_and(|e| e.before_draw(view, canvas, ctx));
    if !busy {
        *export = None;
    }
    busy
}

/// Called after drawing a tile.
pub fn capture(export: &mut Option<ImageExport>, ctx: &egui::Context) {
    if let Some(export) = export {
        export.after_draw(ctx);
    }
}

/// World extents of `rects`, or `None` for an empty diagram.
pub fn bounds(rects: impl IntoIterator<Item = op_core::geom::Rect>) -> Option<(Pos, Pos)> {
    rects.into_iter().fold(None, |acc, r| {
        let (min, max) = acc.unwrap_or((Pos::new(r.x, r.y), Pos::new(r.right(), r.bottom())));
        Some((
            Pos::new(min.x.min(r.x), min.y.min(r.y)),
            Pos::new(max.x.max(r.right()), max.y.max(r.bottom())),
        ))
    })
}

/// Top-left output pixel of each tile, row by row, covering `size`.
fn tiles(size: [usize; 2], tile: [usize; 2]) -> Vec<[usize; 2]> {
    if tile.contains(&0) {
        return vec![];
    }
    (0..size[1])
        .step_by(tile[1])
        .flat_map(|y| (0..size[0]).step_by(tile[0]).map(move |x| [x, y]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_the_whole_image_without_gaps() {
        assert_eq!(
            tiles([250, 120], [100, 100]),
            [[0, 0], [100, 0], [200, 0], [0, 100], [100, 100], [200, 100]]
        );
        assert_eq!(tiles([100, 100], [100, 100]), [[0, 0]]);
        assert!(tiles([10, 10], [0, 4]).is_empty());
    }

    #[test]
    fn large_diagrams_are_scaled_within_the_size_limits() {
        let canvas = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        let small = ImageExport::start(
            Some((Pos::new(0.0, 0.0), Pos::new(420.0, 120.0))),
            View::default(),
            canvas,
            "a".into(),
        );
        assert_eq!((small.scale, small.size), (2.0, [1000, 400]));
        let wide = ImageExport::start(
            Some((Pos::new(0.0, 0.0), Pos::new(20_000.0, 300.0))),
            View::default(),
            canvas,
            "b".into(),
        );
        assert!(wide.size[0] as f32 <= MAX_SIDE && (wide.size[0] * wide.size[1]) as f32 <= MAX_PIXELS + MAX_SIDE);
        assert!(active());
    }

    /// Simulates the screenshots: every tile shows the world point at its corner.
    #[test]
    fn tiles_are_visited_then_the_view_is_restored_and_the_image_is_complete() {
        let ctx = egui::Context::default();
        let canvas = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 200.0));
        let original = View {
            offset: egui::vec2(5.0, 7.0),
            zoom: 0.5,
        };
        let mut view = original;
        let bounds = (Pos::new(100.0, 50.0), Pos::new(400.0, 200.0));
        let mut export = ImageExport::start(Some(bounds), view, canvas, "test.png".into());
        let mut frames = 0;
        while export.before_draw(&mut view, canvas, &ctx) {
            frames += 1;
            assert!(frames < 100);
            let corner = view.to_world(canvas, canvas.min);
            assert!((view.zoom - 2.0).abs() < 1e-6);
            export.after_draw(&ctx);
            let mut shot = ColorImage::filled([400, 300], Color32::BLACK);
            let shade = ((corner.x - export.origin.x) as u8, (corner.y - export.origin.y) as u8);
            for y in 20..220 {
                for x in 10..310 {
                    shot.pixels[y * 400 + x] = Color32::from_rgb(shade.0, shade.1, 255);
                }
            }
            *export.pending.as_ref().unwrap().lock().unwrap() = Some(Arc::new(shot));
        }
        assert_eq!((view.offset, view.zoom), (original.offset, original.zoom), "the view is restored");
        assert_eq!(export.size, [760, 460]);
        assert_eq!(frames, 9, "3 × 3 tiles");
        assert!(export.pixels.iter().all(|p| p.b() == 255), "every pixel comes from a tile");
        // The tile starting 150 world units right of the origin fills x = 300..600.
        assert_eq!(export.pixels[300].r(), 150);
        assert_eq!(export.pixels[200 * 760].g(), 100);
        let (png, name) = crate::io::poll_finished_png().unwrap().unwrap();
        assert_eq!((&png[..4], name.as_str()), (&b"\x89PNG"[..], "test.png"));
    }
}
