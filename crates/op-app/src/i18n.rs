//! Language switch (English / French / Spanish) of the user interface.
//!
//! The translation tables live in [`op_core::i18n`]; this module adds the
//! language dropdown of the top bar and the persistence of the choice.

use egui::{Color32, Rect, Stroke, Ui, pos2, vec2};
use op_core::i18n::{self, Lang, tr};

pub const STORAGE_KEY: &str = "open-process-lang";

pub fn load(storage: Option<&dyn eframe::Storage>) {
    let lang = storage.and_then(|s| eframe::get_value::<Lang>(s, STORAGE_KEY)).unwrap_or_default();
    i18n::set_lang(lang);
}

pub fn save(storage: &mut dyn eframe::Storage) {
    eframe::set_value(storage, STORAGE_KEY, &i18n::lang());
}

/// Dropdown showing the current language and allowing a direct selection.
pub fn language_menu(ui: &mut Ui) {
    let lang = i18n::lang();
    let resp = ui
        .menu_button((flag_icon(lang), format!("{} ▾", lang.code().to_uppercase())), |ui| {
            for choice in Lang::ALL {
                if ui
                    .add(egui::Button::selectable(choice == lang, (flag_icon(choice), choice.native_name())))
                    .clicked()
                {
                    i18n::set_lang(choice);
                    ui.close();
                    ui.ctx().request_repaint();
                }
            }
        })
        .response
        .on_hover_text(tr("ui.language"));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr("ui.language")));
}

fn flag_icon(lang: Lang) -> egui::Atom<'static> {
    egui::Atom::paint(vec2(20.0, 13.0), move |ui, args| paint_flag(ui, lang, args.rect))
}

pub fn paint_flag(ui: &Ui, lang: Lang, r: Rect) {
    let p = ui.painter().with_clip_rect(r);
    match lang {
        Lang::Es => {
            // Spanish flag, with horizontal bands in a 1:2:1 ratio.
            let red = Color32::from_rgb(170, 21, 27);
            let yellow = Color32::from_rgb(241, 191, 0);
            p.rect_filled(r, 0.0, red);
            p.rect_filled(
                Rect::from_min_max(
                    pos2(r.left(), r.top() + r.height() / 4.0),
                    pos2(r.right(), r.bottom() - r.height() / 4.0),
                ),
                0.0,
                yellow,
            );
        }
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
    use super::{STORAGE_KEY, load, save};
    use op_core::i18n::Lang;

    #[test]
    fn language_choice_is_restored() {
        #[derive(Default)]
        struct Storage(std::collections::HashMap<String, String>);
        impl eframe::Storage for Storage {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.to_owned(), value);
            }
            fn remove_string(&mut self, key: &str) {
                self.0.remove(key);
            }
            fn flush(&mut self) {}
        }

        let mut storage = Storage::default();
        for lang in Lang::ALL {
            op_core::i18n::set_lang(lang);
            save(&mut storage);
            assert_eq!(eframe::get_value::<Lang>(&storage, STORAGE_KEY), Some(lang));
            op_core::i18n::set_lang(Lang::En);
            load(Some(&storage));
            assert_eq!(op_core::i18n::lang(), lang);
        }
        load(None);
        assert_eq!(op_core::i18n::lang(), Lang::En);
    }

    /// Stable message keys passed to `tr(`, `trf(` or `key(`.
    fn translatable(src: &str) -> Vec<String> {
        let src = src.split("\n#[cfg(test)]\nmod tests").next().unwrap_or(src);
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
        for (file, src) in crate::test_sources::all() {
            // Catalog values are prose, not call sites. English copy may contain
            // any text, including examples of translation calls.
            if file.contains("/op-core/src/i18n/") {
                continue;
            }
            for message in translatable(&src) {
                used.insert(message.clone());
                if !Lang::En.table().iter().any(|(key, _)| *key == message) {
                    missing.push(format!("{file}: {message:?}"));
                }
            }
        }
        for lang in Lang::ALL {
            let table = lang.table();
            for message in &used {
                if !table.iter().any(|(key, _)| *key == message) {
                    missing.push(format!("{lang:?}: {message:?}"));
                }
            }
            let unused: Vec<_> = table.iter().filter(|(key, _)| !used.contains(*key)).map(|(key, _)| *key).collect();
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
