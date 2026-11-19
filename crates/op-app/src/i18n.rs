//! Language switch (English / French / Spanish) of the user interface.
//!
//! The translation tables live in [`op_core::i18n`]; this module adds the
//! language dropdown of the top bar and the persistence of the choice. Until a
//! language is chosen in the dropdown, each launch follows the system (or
//! browser) language when it is supported, and English otherwise.

use egui::containers::menu::MenuButton;
use egui::{Color32, Rect, Stroke, Ui, pos2, vec2};
use op_core::i18n::{self, Lang, tr};
use std::sync::atomic::{AtomicBool, Ordering};

pub const STORAGE_KEY: &str = "open-process-lang";
/// Set once a language is picked in the dropdown. Older versions saved the
/// current language on every exit, so only a non-English value proves a choice.
pub const EXPLICIT_KEY: &str = "open-process-lang-explicit";

static EXPLICIT: AtomicBool = AtomicBool::new(false);

pub fn load(storage: Option<&dyn eframe::Storage>) {
    let system = if cfg!(test) {
        None
    } else {
        system_language(sys_locale::get_locales())
    };
    let (lang, explicit) = resolve(storage, system);
    EXPLICIT.store(explicit, Ordering::Relaxed);
    i18n::set_lang(lang);
}

/// The language to show, and whether it was chosen in the dropdown.
fn resolve(storage: Option<&dyn eframe::Storage>, system: Option<Lang>) -> (Lang, bool) {
    let saved = storage.and_then(|s| eframe::get_value::<Lang>(s, STORAGE_KEY));
    let explicit = storage
        .and_then(|s| eframe::get_value::<bool>(s, EXPLICIT_KEY))
        .unwrap_or(saved.is_some_and(|lang| lang != Lang::En));
    (if explicit { saved } else { system }.unwrap_or_default(), explicit)
}

/// First supported language among preferences such as `fr-CA` or `es_ES.UTF-8`.
pub fn system_language(locales: impl IntoIterator<Item = String>) -> Option<Lang> {
    locales.into_iter().find_map(|locale| {
        let code = locale.split(['-', '_', '.', '@']).next()?.to_ascii_lowercase();
        Lang::ALL.into_iter().find(|lang| lang.code() == code)
    })
}

pub fn save(storage: &mut dyn eframe::Storage) {
    // Still written for older versions, which read only this key.
    eframe::set_value(storage, STORAGE_KEY, &i18n::lang());
    eframe::set_value(storage, EXPLICIT_KEY, &EXPLICIT.load(Ordering::Relaxed));
}

/// Dropdown showing the current language and allowing a direct selection.
pub fn language_menu(ui: &mut Ui) {
    let lang = i18n::lang();
    let gap = ui.spacing().item_spacing.x;
    let button = egui::Button::new((flag_icon(lang), format!("{} ▾", lang.code().to_uppercase()))).gap(gap);
    let resp = MenuButton::from_button(button)
        .ui(ui, |ui| {
            for choice in Lang::ALL {
                if ui
                    .add(egui::Button::selectable(choice == lang, (flag_icon(choice), choice.native_name())).gap(gap))
                    .clicked()
                {
                    i18n::set_lang(choice);
                    EXPLICIT.store(true, Ordering::Relaxed);
                    ui.close();
                    ui.ctx().request_repaint();
                }
            }
        })
        .0
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
    use super::{EXPLICIT, EXPLICIT_KEY, STORAGE_KEY, load, resolve, save, system_language};
    use op_core::i18n::Lang;
    use std::sync::atomic::Ordering;

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

    #[test]
    fn language_choice_is_restored() {
        let mut storage = Storage::default();
        for lang in Lang::ALL {
            op_core::i18n::set_lang(lang);
            EXPLICIT.store(true, Ordering::Relaxed);
            save(&mut storage);
            assert_eq!(eframe::get_value::<Lang>(&storage, STORAGE_KEY), Some(lang));
            op_core::i18n::set_lang(Lang::En);
            load(Some(&storage));
            assert_eq!(op_core::i18n::lang(), lang);
            // An explicit choice outranks the system language.
            assert_eq!(resolve(Some(&storage), Some(Lang::Es)), (lang, true));
        }
        load(None);
        assert_eq!(op_core::i18n::lang(), Lang::En);
    }

    #[test]
    fn without_a_choice_the_system_language_is_followed() {
        assert_eq!(system_language(["fr-CA".to_string()]), Some(Lang::Fr));
        assert_eq!(system_language(["de-DE", "es_ES.UTF-8", "fr"].map(String::from)), Some(Lang::Es));
        assert_eq!(system_language(["C", "pt-BR"].map(String::from)), None);
        assert_eq!(resolve(None, Some(Lang::Fr)), (Lang::Fr, false));
        assert_eq!(resolve(None, None), (Lang::En, false), "unsupported systems fall back to English");
        // Older versions always saved the language: English proves no choice, others do.
        let mut storage = Storage::default();
        eframe::set_value(&mut storage, STORAGE_KEY, &Lang::En);
        assert_eq!(resolve(Some(&storage), Some(Lang::Es)), (Lang::Es, false));
        eframe::set_value(&mut storage, STORAGE_KEY, &Lang::Fr);
        assert_eq!(resolve(Some(&storage), Some(Lang::Es)), (Lang::Fr, true));
        // Once recorded, the flag decides, including an explicit English.
        eframe::set_value(&mut storage, STORAGE_KEY, &Lang::En);
        eframe::set_value(&mut storage, EXPLICIT_KEY, &true);
        assert_eq!(resolve(Some(&storage), Some(Lang::Es)), (Lang::En, true));
        eframe::set_value(&mut storage, EXPLICIT_KEY, &false);
        assert_eq!(resolve(Some(&storage), Some(Lang::Es)), (Lang::Es, false));
    }

    /// Stable message keys passed to `tr(`, `trf(` or `key(`.
    fn translatable(src: &str) -> Vec<String> {
        // Windows checkouts may use CRLF; keep the test-module boundary detectable.
        let src = src.replace("\r\n", "\n");
        let src = src.split("\n#[cfg(test)]\nmod tests").next().unwrap_or(&src);
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
    fn translatable_ignores_tests_with_lf_and_crlf() {
        let source = concat!(
            "fn production() {\n",
            "    tr(\"file.save\");\n",
            "    trf(\n        \"file.open_failed\", &[\"file\", \"error\"]);\n",
            "    key(\"drawing.choose_tool\");\n",
            "}\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    fn example() { tr(\"test.only\"); }\n",
            "}\n",
        );
        for newline in ["\n", "\r\n"] {
            assert_eq!(
                translatable(&source.replace('\n', newline)),
                ["file.save", "file.open_failed", "drawing.choose_tool"],
                "source with {newline:?} line endings",
            );
        }
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
