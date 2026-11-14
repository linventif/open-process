//! Rendering and rasterization run in a native thread or the shared Wasm worker.
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Serialize, Deserialize)]
pub(super) struct Input {
    pub task: String,
    pub source: String,
    #[serde(default)]
    pub lang: op_core::i18n::Lang,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Preview {
    pub svg: String,
    pub png: Vec<u8>,
    pub width: f32,
    pub height: f32,
}

pub(crate) fn compute_json(input: String, stop: Arc<AtomicBool>) -> Result<String, String> {
    let input: Input = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    op_core::i18n::with_lang(input.lang, || {
        if input.source.len() > 256 * 1024 {
            return Err(op_core::i18n::tr("file.input_limit").into());
        }
        let svg = op_core::mermaid::render_svg(&input.source)?;
        if stop.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let preview = rasterize(svg)?;
        serde_json::to_string(&preview).map_err(|e| e.to_string())
    })
}

fn rasterize(svg: String) -> Result<Preview, String> {
    use resvg::{tiny_skia, usvg};
    let mut options = usvg::Options {
        font_family: "Inter".into(),
        ..Default::default()
    };
    let fonts = options.fontdb_mut();
    fonts.load_font_data(include_bytes!("../../assets/fonts/Inter-Regular.ttf").to_vec());
    fonts.load_font_data(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf").to_vec());
    fonts.set_sans_serif_family("Inter");
    fonts.set_serif_family("Inter");
    fonts.set_monospace_family("Inter");
    let tree = usvg::Tree::from_str(&svg, &options).map_err(|e| e.to_string())?;
    let width = tree.size().width();
    let height = tree.size().height();
    // Bound allocation, including unusually wide/tall diagrams. Keep logical SVG dimensions for zoom.
    let scale = (2048.0 / width.max(height)).min(2.0);
    let mut pixmap = tiny_skia::Pixmap::new((width * scale).ceil().max(1.0) as u32, (height * scale).ceil().max(1.0) as u32)
        .ok_or("Cannot allocate Mermaid preview")?;
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|e| e.to_string())?;
    Ok(Preview { svg, png, width, height })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_contains_svg_and_a_bounded_png_with_embedded_fonts() {
        let input = Input {
            task: "mermaid.preview".into(),
            source: op_core::mermaid::TEMPLATES[0].source.into(),
            lang: op_core::i18n::Lang::En,
        };
        let raw = compute_json(serde_json::to_string(&input).unwrap(), Arc::new(AtomicBool::new(false))).unwrap();
        let preview: Preview = serde_json::from_str(&raw).unwrap();
        assert!(preview.svg.contains("Process order"));
        let img = image::load_from_memory(&preview.png).unwrap().into_rgba8();
        assert!(img.width() <= 2048 && img.height() <= 2048);
        assert!(img.pixels().any(|p| p.0 != [255; 4]));
    }
}
