//! Visual identity: fonts (Inter), colors, spacing, rounding and shadows
//! for both light and dark themes.

use egui::epaint::Shadow;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle, Theme, Visuals, vec2};
use std::sync::Arc;

pub const SEMIBOLD: &str = "semibold";
pub const MEDIUM: &str = "medium";
pub const HAND: &str = "hand";

pub const CONTROL_RADIUS: u8 = 7;
pub const CARD_RADIUS: u8 = 12;
pub const BUTTON_RADIUS: u8 = 8;

/// Brand accent.
pub const ACCENT: Color32 = Color32::from_rgb(76, 110, 245);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(120, 146, 255);

/// Badge colors of the document kinds.
pub const KIND_BPMN: Color32 = Color32::from_rgb(59, 130, 246);
pub const KIND_PETRI: Color32 = Color32::from_rgb(139, 92, 246);
pub const KIND_LOG: Color32 = Color32::from_rgb(217, 119, 6);

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Handwriting font used by the sketch mode.
pub fn hand(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(HAND.into()))
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}

pub fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode { ACCENT_DARK } else { ACCENT }
}

pub fn accent_text(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(20, 22, 26)
    } else {
        Color32::WHITE
    }
}

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut fonts, "Inter", include_bytes!("../assets/fonts/Inter-Regular.ttf"));
    add(&mut fonts, "Inter-Medium", include_bytes!("../assets/fonts/Inter-Medium.ttf"));
    add(&mut fonts, "Inter-SemiBold", include_bytes!("../assets/fonts/Inter-SemiBold.ttf"));
    add(&mut fonts, "PatrickHand", include_bytes!("../assets/fonts/PatrickHand-Regular.ttf"));

    // Fallbacks shared by every family: Hack (arrows, math) then egui's icon/emoji fonts.
    let fallbacks: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let family = |main: &str| -> Vec<String> {
        let mut v = vec![main.to_owned(), "Hack".to_owned()];
        v.extend(fallbacks.iter().filter(|f| *f != "Ubuntu-Light" && *f != "Hack").cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, family("Inter"));
    fonts.families.insert(FontFamily::Name(MEDIUM.into()), family("Inter-Medium"));
    fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), family("Inter-SemiBold"));
    fonts.families.insert(FontFamily::Name(HAND.into()), family("PatrickHand"));
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(1, "Inter".to_owned());
    }
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    setup_fonts(ctx);
    for theme in [Theme::Light, Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            style.text_styles = [
                (TextStyle::Small, FontId::proportional(11.5)),
                (TextStyle::Body, FontId::proportional(13.5)),
                (TextStyle::Button, FontId::proportional(13.5)),
                (TextStyle::Heading, semibold(17.0)),
                (TextStyle::Monospace, FontId::monospace(12.5)),
            ]
            .into();
            let sp = &mut style.spacing;
            sp.item_spacing = vec2(8.0, 6.0);
            sp.button_padding = vec2(10.0, 4.0);
            sp.interact_size = vec2(36.0, 26.0);
            sp.menu_margin = Margin::same(8);
            sp.window_margin = Margin::same(12);
            sp.indent = 16.0;
            sp.combo_width = 140.0;
            let dark = theme == Theme::Dark;
            visuals(&mut style.visuals, dark);
        });
    }
}

fn visuals(v: &mut Visuals, dark: bool) {
    let c = |r, g, b| Color32::from_rgb(r, g, b);
    let accent = if dark { ACCENT_DARK } else { ACCENT };
    let (panel, window, extreme, border, text, weak, inactive, hovered, active) = if dark {
        (
            c(28, 30, 35),
            c(35, 38, 44),
            c(20, 22, 26),
            c(50, 54, 62),
            c(228, 231, 236),
            c(150, 156, 168),
            c(44, 48, 55),
            c(54, 59, 68),
            c(62, 74, 120),
        )
    } else {
        (
            c(246, 247, 249),
            c(255, 255, 255),
            c(255, 255, 255),
            c(225, 228, 234),
            c(30, 34, 40),
            c(105, 112, 125),
            c(236, 238, 242),
            c(225, 229, 236),
            c(214, 222, 252),
        )
    };
    v.dark_mode = dark;
    v.panel_fill = panel;
    v.window_fill = window;
    v.extreme_bg_color = extreme;
    v.text_edit_bg_color = Some(extreme);
    v.faint_bg_color = if dark { c(33, 36, 42) } else { c(242, 244, 247) };
    v.code_bg_color = if dark { c(40, 44, 52) } else { c(238, 240, 244) };
    v.hyperlink_color = accent;
    v.weak_text_color = Some(weak);
    v.override_text_color = Some(text);
    v.window_stroke = Stroke::new(1.0, border);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    let shadow_color = Color32::from_black_alpha(if dark { 110 } else { 28 });
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: shadow_color,
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: shadow_color,
    };
    v.selection.bg_fill = accent.gamma_multiply(if dark { 0.45 } else { 0.22 });
    v.selection.stroke = Stroke::new(1.5, accent);
    v.striped = false;
    v.collapsing_header_frame = false;
    v.indent_has_left_vline = false;

    let r = CornerRadius::same(7);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = window;
    w.noninteractive.weak_bg_fill = panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = inactive;
    w.inactive.weak_bg_fill = inactive;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = hovered;
    w.hovered.weak_bg_fill = hovered;
    w.hovered.bg_stroke = Stroke::new(1.0, border);
    w.hovered.fg_stroke = Stroke::new(1.2, text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = active;
    w.active.weak_bg_fill = active;
    w.active.bg_stroke = Stroke::new(1.0, accent);
    w.active.fg_stroke = Stroke::new(1.5, text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open.bg_fill = hovered;
    w.open.weak_bg_fill = hovered;
    w.open.bg_stroke = Stroke::new(1.0, border);
    w.open.fg_stroke = Stroke::new(1.0, text);
    w.open.corner_radius = r;
}

/// Shared popup appearance, retaining readable padding inside menu rows.
pub fn menu_style(style: &mut egui::Style) {
    egui::containers::menu::menu_style(style);
    style.spacing.button_padding = vec2(10.0, 4.0);
}

/// Shared quiet buttons for navigation, view configuration and inspector tabs.
/// Keep the strong background for checkboxes and the accent outline for keyboard focus.
pub fn toolbar_style(style: &mut egui::Style) {
    let hover = style.visuals.widgets.hovered.bg_fill;
    let selected = style.visuals.selection.bg_fill;
    let accent = style.visuals.selection.stroke.color;
    style.spacing.button_padding = vec2(6.0, 4.0);
    style.visuals.selection.bg_fill = selected;
    style.visuals.selection.stroke = Stroke::new(1.0, accent);
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = Stroke::NONE;
    for state in [&mut widgets.hovered, &mut widgets.open] {
        state.bg_fill = hover;
        state.weak_bg_fill = hover;
        state.bg_stroke = Stroke::NONE;
    }
    let radius = widgets.noninteractive.corner_radius;
    for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
        state.corner_radius = radius;
        state.fg_stroke.width = 1.0;
        state.expansion = 0.0;
    }
}

/// Small uppercase section title used in side panels.
pub fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(title.chars().map(|c| c.to_ascii_uppercase()).collect::<String>())
            .font(semibold(10.5))
            .color(ui.visuals().weak_text_color())
            .extra_letter_spacing(0.6),
    );
    ui.add_space(2.0);
}

/// A rounded "card" frame (for grouped content).
pub fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(16))
}

/// Segmented control (pill tabs). Returns true if the value changed.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(ui.visuals().widgets.inactive.bg_fill)
        .corner_radius(CornerRadius::same(CONTROL_RADIUS))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for &(v, label) in options {
                    let selected = *value == v;
                    let text = egui::RichText::new(label).font(medium(12.5));
                    let fill = if selected { ui.visuals().window_fill } else { Color32::TRANSPARENT };
                    let btn = egui::Button::new(text)
                        .fill(fill)
                        .corner_radius(CornerRadius::same(CONTROL_RADIUS))
                        .stroke(if selected {
                            Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
                        } else {
                            Stroke::NONE
                        });
                    let r = ui.add(btn);
                    if selected && !dark {
                        // subtle lift
                        ui.painter().rect_stroke(
                            r.rect,
                            7.0,
                            Stroke::new(1.0, Color32::from_black_alpha(10)),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if r.clicked() && !selected {
                        *value = v;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Filled accent button for primary actions.
pub fn primary_button<'a>(ui: &egui::Ui, text: &str) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(text.to_owned()).font(semibold(13.0)).color(accent_text(ui)))
        .fill(accent(ui))
        .corner_radius(CornerRadius::same(BUTTON_RADIUS))
        .min_size(vec2(0.0, 30.0))
}

/// Explicit web links can select a color mode; retired theme names are ignored.
#[cfg(any(target_arch = "wasm32", test))]
pub fn apply_color_mode_query(ctx: &egui::Context, search: &str) {
    for entry in search.trim_start_matches('?').split('&') {
        match entry.split_once('=') {
            Some(("mode", "dark")) => ctx.set_theme(Theme::Dark),
            Some(("mode", "light")) => ctx.set_theme(Theme::Light),
            _ => {}
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_web_color_mode(ctx: &egui::Context) {
    if let Some(search) = web_sys::window().and_then(|w| w.location().search().ok()) {
        apply_color_mode_query(ctx, &search);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn retired_theme_links_cannot_change_classic_or_override_the_color_mode() {
        let ctx = egui::Context::default();
        super::apply(&ctx);
        let light = ctx.style_of(egui::Theme::Light).visuals.clone();
        let dark = ctx.style_of(egui::Theme::Dark).visuals.clone();
        super::apply_color_mode_query(&ctx, "?theme=cloud&mode=dark");
        assert_eq!(ctx.options(|o| o.theme_preference), egui::ThemePreference::Dark);
        super::apply_color_mode_query(&ctx, "?theme=workshop&mode=unknown");
        assert_eq!(ctx.options(|o| o.theme_preference), egui::ThemePreference::Dark);
        super::apply_color_mode_query(&ctx, "?theme=midnight&mode=light");
        assert_eq!(ctx.options(|o| o.theme_preference), egui::ThemePreference::Light);
        assert_eq!(ctx.style_of(egui::Theme::Light).visuals.panel_fill, light.panel_fill);
        assert_eq!(ctx.style_of(egui::Theme::Dark).visuals.panel_fill, dark.panel_fill);
        assert_eq!(ctx.style_of(egui::Theme::Light).visuals.selection.stroke, light.selection.stroke);
    }
}
