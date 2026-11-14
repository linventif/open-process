//! Appearance preferences are scoped to an egui context, outside document state.

use egui::{Color32, Context, Ui};
use op_core::i18n::tr;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const STORAGE_KEY: &str = "open-process-appearance-v1";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PresetId {
    #[default]
    Classic,
    Cloud,
    Lilac,
    Blossom,
    Lagoon,
    Midnight,
    Graphite,
    Workshop,
}

impl PresetId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => tr("appearance.classic"),
            Self::Cloud => tr("appearance.cloud"),
            Self::Lilac => tr("appearance.lilac"),
            Self::Blossom => tr("appearance.blossom"),
            Self::Lagoon => tr("appearance.lagoon"),
            Self::Midnight => tr("appearance.midnight"),
            Self::Graphite => tr("appearance.graphite"),
            Self::Workshop => tr("appearance.workshop"),
        }
    }

    #[cfg(test)]
    pub fn slug(self) -> String {
        serde_json::to_value(self).unwrap().as_str().unwrap().to_owned()
    }
}

#[derive(Clone, Copy, Deserialize)]
pub struct Colors {
    pub background: [u8; 3],
    pub surface: [u8; 3],
    pub canvas: [u8; 3],
    pub text: [u8; 3],
    pub muted: [u8; 3],
    pub border: [u8; 3],
    pub accent: [u8; 3],
    pub on_accent: [u8; 3],
}

pub fn color(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

#[derive(Deserialize)]
pub struct Preset {
    pub id: PresetId,
    pub light: Colors,
    pub dark: Colors,
    pub control_radius: u8,
    pub card_radius: u8,
    pub button_radius: u8,
    pub shadow_blur: u8,
}

impl Preset {
    pub fn colors(&self, dark: bool) -> Colors {
        if dark { self.dark } else { self.light }
    }
}

pub fn all() -> &'static [Preset] {
    static PRESETS: OnceLock<Vec<Preset>> = OnceLock::new();
    PRESETS.get_or_init(|| serde_json::from_str(include_str!("../../assets/appearance.json")).expect("embedded appearance catalogue"))
}

fn context_key() -> egui::Id {
    egui::Id::unique(STORAGE_KEY)
}

pub fn active(ctx: &Context) -> PresetId {
    ctx.data(|data| data.get_temp(context_key())).unwrap_or_default()
}

pub fn current(ctx: &Context) -> &'static Preset {
    let id = active(ctx);
    all().iter().find(|p| p.id == id).expect("registered appearance")
}

pub fn set(ctx: &Context, id: PresetId) {
    ctx.data_mut(|data| data.insert_temp(context_key(), id));
    super::apply_styles(ctx);
    ctx.request_repaint();
}

pub fn load(ctx: &Context, storage: Option<&dyn eframe::Storage>) {
    let id = storage.and_then(|s| eframe::get_value(s, STORAGE_KEY)).unwrap_or_default();
    set(ctx, id);
    #[cfg(target_arch = "wasm32")]
    if let Some(search) = web_sys::window().and_then(|w| w.location().search().ok()) {
        apply_query(ctx, &search);
    }
}

pub fn save(ctx: &Context, storage: &mut dyn eframe::Storage) {
    eframe::set_value(storage, STORAGE_KEY, &active(ctx));
}

#[cfg(any(target_arch = "wasm32", test))]
pub fn apply_query(ctx: &Context, search: &str) {
    for entry in search.trim_start_matches('?').split('&') {
        match entry.split_once('=') {
            Some(("theme", id)) => {
                if let Ok(id) = serde_json::from_value(serde_json::Value::String(id.into())) {
                    set(ctx, id);
                }
            }
            Some(("mode", "dark")) => ctx.set_theme(egui::Theme::Dark),
            Some(("mode", "light")) => ctx.set_theme(egui::Theme::Light),
            _ => {}
        }
    }
}

pub fn menu(ui: &mut Ui) {
    let selected = active(ui.ctx());
    let response = ui
        .menu_button(selected.label(), |ui| {
            super::menu_style(ui.style_mut());
            ui.label(tr("appearance.themes"));
            ui.separator();
            for preset in all() {
                if ui.selectable_label(preset.id == selected, preset.id.label()).clicked() {
                    set(ui.ctx(), preset.id);
                    if preset.id == PresetId::Midnight {
                        ui.ctx().set_theme(egui::Theme::Dark);
                    }
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr("appearance.themes"));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr("appearance.themes")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage as _;

    #[derive(Default)]
    struct Storage(std::collections::HashMap<String, String>);
    impl eframe::Storage for Storage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn preferences_restore_without_leaking_to_other_contexts_and_recover_invalid_data() {
        let first = Context::default();
        let other = Context::default();
        let mut storage = Storage::default();
        for preset in all() {
            set(&first, preset.id);
            save(&first, &mut storage);
            let restored = Context::default();
            load(&restored, Some(&storage));
            assert_eq!(active(&restored), preset.id);
            assert_eq!(active(&other), PresetId::Classic);
        }
        for invalid in ["not-json", "\"unknown-future-theme\"", "null", "{}"] {
            storage.set_string(STORAGE_KEY, invalid.into());
            load(&first, Some(&storage));
            assert_eq!(active(&first), PresetId::Classic);
        }
        load(&first, None);
        assert_eq!(active(&first), PresetId::Classic);
    }

    #[test]
    fn explicit_browser_links_override_only_known_preferences() {
        let ctx = Context::default();
        apply_query(&ctx, "?theme=cloud&mode=dark&unrelated=1");
        assert_eq!(active(&ctx), PresetId::Cloud);
        assert_eq!(ctx.options(|o| o.theme_preference), egui::ThemePreference::Dark);
        apply_query(&ctx, "?theme=unknown&mode=invalid");
        assert_eq!(active(&ctx), PresetId::Cloud);
        assert_eq!(ctx.options(|o| o.theme_preference), egui::ThemePreference::Dark);
        apply_query(&ctx, "?theme=classic&mode=light");
        assert_eq!(active(&ctx), PresetId::Classic);
        assert_eq!(ctx.options(|o| o.theme_preference), egui::ThemePreference::Light);
    }
}
