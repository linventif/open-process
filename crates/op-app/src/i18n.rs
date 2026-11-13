//! Language switch (English / French) of the user interface.
//!
//! The translation tables live in [`op_core::i18n`]; this module adds the
//! painted flag button of the top bar and the persistence of the choice.

use egui::{Color32, Rect, Stroke, Ui, pos2, vec2};
use op_core::i18n::{self, Lang, tr, trf};

pub const STORAGE_KEY: &str = "open-process-lang";

pub fn load(storage: Option<&dyn eframe::Storage>) {
    let lang = storage.and_then(|s| eframe::get_value::<Lang>(s, STORAGE_KEY)).unwrap_or_default();
    i18n::set_lang(lang);
}

pub fn save(storage: &mut dyn eframe::Storage) {
    eframe::set_value(storage, STORAGE_KEY, &i18n::lang());
}

/// Flag button showing the current language; a click switches to the next one.
pub fn lang_toggle(ui: &mut Ui) {
    let lang = i18n::lang();
    let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 7.0, ui.visuals().widgets.hovered.bg_fill);
    }
    let flag = Rect::from_center_size(rect.center(), vec2(20.0, 13.0));
    paint_flag(ui, lang, flag);
    let next = lang.next();
    let resp = resp.on_hover_text(trf("Language: {} → {}", &[&lang.native_name(), &next.native_name()]));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr("Language")));
    if resp.clicked() {
        i18n::set_lang(next);
        ui.ctx().request_repaint();
    }
}

pub fn paint_flag(ui: &Ui, lang: Lang, r: Rect) {
    let p = ui.painter().with_clip_rect(r);
    match lang {
        Lang::Fr => {
            let w = r.width() / 3.0;
            for (k, c) in [Color32::from_rgb(0, 85, 164), Color32::WHITE, Color32::from_rgb(239, 65, 53)]
                .into_iter()
                .enumerate()
            {
                let x = r.left() + w * k as f32;
                p.rect_filled(Rect::from_min_max(pos2(x, r.top()), pos2(x + w, r.bottom())), 0.0, c);
            }
        }
        Lang::En => {
            // Simplified Union Jack.
            let blue = Color32::from_rgb(1, 33, 105);
            let red = Color32::from_rgb(200, 16, 46);
            p.rect_filled(r, 0.0, blue);
            let diag = [[r.left_top(), r.right_bottom()], [r.left_bottom(), r.right_top()]];
            for d in diag {
                p.line_segment(d, Stroke::new(3.0, Color32::WHITE));
            }
            for d in diag {
                p.line_segment(d, Stroke::new(1.0, red));
            }
            let c = r.center();
            p.rect_filled(Rect::from_center_size(c, vec2(r.width(), 4.4)), 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_center_size(c, vec2(4.4, r.height())), 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_center_size(c, vec2(r.width(), 2.4)), 0.0, red);
            p.rect_filled(Rect::from_center_size(c, vec2(2.4, r.height())), 0.0, red);
        }
    }
    ui.painter().rect_stroke(
        r,
        1.5,
        Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.6)),
        egui::StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use op_core::i18n::{Lang, placeholders};

    /// Every source file whose texts go through `tr` / `trf` / `key`.
    const SOURCES: &[(&str, &str)] = &[
        ("app.rs", include_str!("app.rs")),
        ("bpmn_editor.rs", include_str!("bpmn_editor.rs")),
        ("canvas.rs", include_str!("canvas.rs")),
        ("dock.rs", include_str!("dock.rs")),
        ("i18n.rs", include_str!("i18n.rs")),
        ("io.rs", include_str!("io.rs")),
        ("mining_editor.rs", include_str!("mining_editor.rs")),
        ("petri_editor.rs", include_str!("petri_editor.rs")),
        ("theme.rs", include_str!("theme.rs")),
        ("analysis.rs", include_str!("../../op-core/src/analysis.rs")),
        ("answers.rs", include_str!("../../op-core/src/answers.rs")),
        ("bpmn.rs", include_str!("../../op-core/src/bpmn.rs")),
        ("convert.rs", include_str!("../../op-core/src/convert.rs")),
        ("examples.rs", include_str!("../../op-core/src/examples.rs")),
        ("evaluation.rs", include_str!("../../op-core/src/evaluation.rs")),
        ("mining.rs", include_str!("../../op-core/src/mining.rs")),
        ("petri.rs", include_str!("../../op-core/src/petri.rs")),
        ("style.rs", include_str!("../../op-core/src/style.rs")),
    ];

    /// The string literals passed to `tr(`, `trf(` or `key(`.
    fn translatable(src: &str) -> Vec<String> {
        let src = src.split("\n#[cfg(test)]").next().unwrap_or(src);
        let mut out = Vec::new();
        let b = src.as_bytes();
        for pat in ["tr(", "trf(", "key("] {
            let mut from = 0;
            while let Some(i) = src[from..].find(pat) {
                let at = from + i;
                from = at + pat.len();
                if at > 0 && (b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_') {
                    continue;
                }
                // The literal may start on the next line.
                let ws = src[from..].len() - src[from..].trim_start().len();
                if !src[from + ws..].starts_with('"') {
                    continue;
                }
                from += ws + 1;
                let mut s = String::new();
                let mut chars = src[from..].chars();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some(o) => s.push(o),
                            None => {}
                        },
                        c => s.push(c),
                    }
                }
                out.push(s);
            }
        }
        out
    }

    #[test]
    fn every_text_is_translated() {
        let mut missing = Vec::new();
        let mut used = std::collections::HashSet::new();
        for lang in Lang::ALL {
            let Some(table) = lang.table() else { continue };
            let map: std::collections::HashMap<&str, &str> = table.iter().copied().collect();
            for (file, src) in SOURCES {
                for s in translatable(src) {
                    used.insert(s.clone());
                    match map.get(s.as_str()) {
                        None => missing.push(format!("{lang:?} {file}: {s:?}")),
                        Some(t) => assert_eq!(placeholders(&s), placeholders(t), "{s:?}"),
                    }
                }
            }
            let unused: Vec<_> = table.iter().filter(|(en, _)| !used.contains(*en)).map(|(en, _)| *en).collect();
            assert!(unused.is_empty(), "{lang:?}: unused translations: {unused:#?}");
        }
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "{} missing translations:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }
}
