//! Open Process workshop identity: Inter, paper, graphite and vermilion
//! for both light and dark themes.

use egui::epaint::Shadow;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle, Theme, Visuals, vec2};
use std::sync::Arc;

pub const SEMIBOLD: &str = "semibold";
pub const MEDIUM: &str = "medium";
pub const HAND: &str = "hand";

/// Identity tokens shared with the website; see docs/identite-graphique.md.
pub const PAPER: Color32 = Color32::from_rgb(243, 241, 232);
pub const SHEET: Color32 = Color32::from_rgb(252, 251, 247);
pub const INK: Color32 = Color32::from_rgb(37, 41, 35);
pub const ACCENT: Color32 = Color32::from_rgb(196, 71, 45);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(245, 139, 111);
pub const BORDER: Color32 = Color32::from_rgb(199, 201, 188);
pub const MUTED: Color32 = Color32::from_rgb(97, 103, 95);

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

pub fn accent(dark: bool) -> Color32 {
    if dark { ACCENT_DARK } else { ACCENT }
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
            visuals(&mut style.visuals, theme == Theme::Dark);
        });
    }
}

fn visuals(v: &mut Visuals, dark: bool) {
    let c = |r, g, b| Color32::from_rgb(r, g, b);
    let accent = accent(dark);
    let (panel, window, extreme, border, text, weak, inactive, hovered, active) = if dark {
        (
            c(35, 39, 34),
            c(43, 48, 41),
            c(27, 31, 27),
            c(76, 85, 72),
            PAPER,
            c(182, 188, 174),
            c(52, 59, 49),
            c(64, 72, 60),
            c(85, 57, 44),
        )
    } else {
        (
            PAPER,
            SHEET,
            SHEET,
            BORDER,
            INK,
            MUTED,
            c(232, 231, 220),
            c(222, 224, 211),
            c(244, 221, 208),
        )
    };
    v.dark_mode = dark;
    v.panel_fill = panel;
    v.window_fill = window;
    v.extreme_bg_color = extreme;
    v.text_edit_bg_color = Some(extreme);
    v.faint_bg_color = if dark { c(39, 44, 37) } else { c(237, 236, 225) };
    v.code_bg_color = if dark { c(47, 54, 44) } else { c(232, 231, 220) };
    v.hyperlink_color = accent;
    v.weak_text_color = Some(weak);
    v.override_text_color = Some(text);
    v.window_stroke = Stroke::new(1.0, border);
    v.window_corner_radius = CornerRadius::same(3);
    v.menu_corner_radius = CornerRadius::same(3);
    let shadow_color = Color32::from_black_alpha(if dark { 110 } else { 28 });
    v.window_shadow = Shadow {
        offset: [3, 4],
        blur: 0,
        spread: 0,
        color: shadow_color,
    };
    v.popup_shadow = Shadow {
        offset: [2, 3],
        blur: 0,
        spread: 0,
        color: shadow_color,
    };
    v.selection.bg_fill = accent.gamma_multiply(if dark { 0.45 } else { 0.22 });
    v.selection.stroke = Stroke::new(1.5, accent);
    v.striped = false;
    v.collapsing_header_frame = false;
    v.indent_has_left_vline = false;

    let r = CornerRadius::same(3);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = window;
    w.noninteractive.weak_bg_fill = panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = inactive;
    w.inactive.weak_bg_fill = inactive;
    w.inactive.bg_stroke = Stroke::new(1.0, border);
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
    let dark = style.visuals.dark_mode;
    let hover = if dark {
        Color32::from_rgb(64, 72, 60)
    } else {
        Color32::from_rgb(222, 224, 211)
    };
    let selected = if dark {
        Color32::from_rgb(85, 57, 44)
    } else {
        Color32::from_rgb(244, 221, 208)
    };
    style.spacing.button_padding = vec2(6.0, 4.0);
    style.visuals.selection.bg_fill = selected;
    style.visuals.selection.stroke = Stroke::new(1.0, accent(dark));
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = Stroke::NONE;
    for state in [&mut widgets.hovered, &mut widgets.open] {
        state.bg_fill = hover;
        state.weak_bg_fill = hover;
        state.bg_stroke = Stroke::NONE;
    }
    for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active, &mut widgets.open] {
        state.corner_radius = CornerRadius::same(3);
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

/// A ruled paper frame for grouped content.
pub fn card(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::same(16))
}

/// Segmented control. Returns true if the value changed.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(ui.visuals().widgets.inactive.bg_fill)
        .corner_radius(CornerRadius::same(3))
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
                        .corner_radius(CornerRadius::same(2))
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
                            2.0,
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
    egui::Button::new(
        egui::RichText::new(text.to_owned())
            .font(semibold(13.0))
            .color(if ui.visuals().dark_mode { INK } else { SHEET }),
    )
    .fill(accent(ui.visuals().dark_mode))
    .corner_radius(CornerRadius::same(3))
    .min_size(vec2(0.0, 30.0))
}
