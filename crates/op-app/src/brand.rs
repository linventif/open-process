//! Shared application identity for the native window and egui views.

use std::sync::OnceLock;

pub const ICON_PNG: &[u8] = include_bytes!("../assets/icons/open-process.png");
const LIGHT_ICON_PNG: &[u8] = include_bytes!("../assets/icons/open-process-light.png");

pub fn show(ui: &mut egui::Ui, size: f32) {
    let dark = ui.visuals().dark_mode;
    let id = egui::Id::unique(("open_process_brand_icon", dark));
    let cached = ui.ctx().data_mut(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = cached.unwrap_or_else(|| {
        static IMAGE: OnceLock<egui::ColorImage> = OnceLock::new();
        static LIGHT_IMAGE: OnceLock<egui::ColorImage> = OnceLock::new();
        let cache = if dark { &LIGHT_IMAGE } else { &IMAGE };
        let image = cache.get_or_init(|| {
            let bytes = if dark { LIGHT_ICON_PNG } else { ICON_PNG };
            let rgba = image::load_from_memory(bytes).expect("embedded application icon").into_rgba8();
            egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw())
        });
        let texture = ui
            .ctx()
            .load_texture("open-process-icon", image.clone(), egui::TextureOptions::LINEAR);
        ui.ctx().data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    ui.image((texture.id(), egui::vec2(size, size)));
}
