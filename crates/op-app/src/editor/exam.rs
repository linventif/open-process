//! Application-wide examination preference, separate from document content.

pub fn active(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(egui::Id::unique("exam_mode")).unwrap_or(false))
}

pub fn set_active(ctx: &egui::Context, active: bool) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("exam_mode"), active));
}
