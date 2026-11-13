//! User-interface translations.
//!
//! English is the source language: every text is written in English in the
//! code and wrapped in [`tr`] (or [`trf`] when it has `{}` placeholders).
//! Other languages are lookup tables keyed by the English text; a missing
//! entry falls back to English.
//!
//! Texts kept in tables and translated later (palette entries, labels…) are
//! marked with [`key`] so the completeness test can find them.

mod es;
mod fr;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Write as _};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    En,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Fr, Lang::Es];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
        }
    }

    /// Name of the language, written in that language.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Fr => "Français",
            Lang::Es => "Español",
        }
    }

    /// The next language (for a toggle button).
    pub fn next(self) -> Lang {
        let i = Lang::ALL.iter().position(|&l| l == self).unwrap_or(0);
        Lang::ALL[(i + 1) % Lang::ALL.len()]
    }

    /// The translation table (`None` for English, the source language).
    pub fn table(self) -> Option<&'static [(&'static str, &'static str)]> {
        match self {
            Lang::En => None,
            Lang::Fr => Some(fr::TABLE),
            Lang::Es => Some(es::TABLE),
        }
    }
}

static LANG: AtomicU8 = AtomicU8::new(0);

/// Current user-interface language.
pub fn lang() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        1 => Lang::Fr,
        2 => Lang::Es,
        _ => Lang::En,
    }
}

pub fn set_lang(lang: Lang) {
    LANG.store(
        match lang {
            Lang::En => 0,
            Lang::Fr => 1,
            Lang::Es => 2,
        },
        Ordering::Relaxed,
    );
}

/// Marks an English text that is translated later with [`tr`]. Identity.
pub const fn key(en: &'static str) -> &'static str {
    en
}

/// Translates an English text into the current language.
pub fn tr(en: &'static str) -> &'static str {
    translate(lang(), en)
}

/// Translates an English text into the given language.
pub fn translate(lang: Lang, en: &'static str) -> &'static str {
    static FR: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static ES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    match lang {
        Lang::En => en,
        Lang::Fr => FR
            .get_or_init(|| fr::TABLE.iter().copied().collect())
            .get(en)
            .copied()
            .unwrap_or(en),
        Lang::Es => ES
            .get_or_init(|| es::TABLE.iter().copied().collect())
            .get(en)
            .copied()
            .unwrap_or(en),
    }
}

/// Translates a text with placeholders, then fills them: `{}` takes the next
/// argument, `{0}`, `{1}`… a given one (translations may reorder them).
pub fn trf(en: &'static str, args: &[&dyn Display]) -> String {
    fill(tr(en), args)
}

pub fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) if after[..close].chars().all(|c| c.is_ascii_digit()) => {
                let idx = if close == 0 {
                    next += 1;
                    next - 1
                } else {
                    after[..close].parse().unwrap_or(usize::MAX)
                };
                if let Some(a) = args.get(idx) {
                    let _ = write!(out, "{a}");
                }
                rest = &after[close + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Number of argument slots used by a template (for consistency checks).
pub fn placeholders(template: &str) -> Vec<usize> {
    let mut v = Vec::new();
    let mut next = 0;
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) if after[..close].chars().all(|c| c.is_ascii_digit()) => {
                v.push(if close == 0 {
                    next += 1;
                    next - 1
                } else {
                    after[..close].parse().unwrap_or(usize::MAX)
                });
                rest = &after[close + 1..];
            }
            _ => rest = after,
        }
    }
    v.sort_unstable();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_cycle_and_serialization() {
        assert_eq!(Lang::Es.code(), "es");
        assert_eq!(Lang::Es.native_name(), "Español");
        assert_eq!(Lang::En.next(), Lang::Fr);
        assert_eq!(Lang::Fr.next(), Lang::Es);
        assert_eq!(Lang::Es.next(), Lang::En);
        for (lang, stored) in [(Lang::En, "\"En\""), (Lang::Fr, "\"Fr\""), (Lang::Es, "\"Es\"")] {
            assert_eq!(serde_json::to_string(&lang).unwrap(), stored);
            assert_eq!(serde_json::from_str::<Lang>(stored).unwrap(), lang);
        }
    }

    #[test]
    fn fill_placeholders() {
        assert_eq!(fill("{} of {}", &[&1, &2]), "1 of 2");
        assert_eq!(fill("{1} before {0}", &[&"a", &"b"]), "b before a");
        assert_eq!(fill("set {a} {}", &[&3]), "set {a} 3");
        assert_eq!(placeholders("{1} x {0}"), vec![0, 1]);
    }

    #[test]
    fn fallback_and_lookup() {
        assert_eq!(translate(Lang::En, "Save"), "Save");
        assert_eq!(translate(Lang::Fr, "Save"), "Enregistrer");
        assert_eq!(translate(Lang::Fr, "no such text 42"), "no such text 42");
    }

    #[test]
    fn tables_are_consistent() {
        for lang in Lang::ALL {
            let Some(table) = lang.table() else { continue };
            let mut seen = std::collections::HashSet::new();
            for (en, t) in table {
                assert!(seen.insert(*en), "{lang:?}: duplicate entry {en:?}");
                assert!(!t.is_empty(), "{lang:?}: empty translation of {en:?}");
                assert_eq!(
                    placeholders(en),
                    placeholders(t),
                    "{lang:?}: placeholders differ in {en:?} -> {t:?}"
                );
                assert_eq!(
                    en.matches('\n').count(),
                    t.matches('\n').count(),
                    "{lang:?}: line count differs in {en:?}"
                );
            }
        }
    }
}
