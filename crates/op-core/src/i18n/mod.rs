//! User-interface translations.
//!
//! Messages use stable keys such as `file.save` and `drawing.choose_tool`.
//! English copy lives in `en.rs`; editing it never changes the message key.
//! [`tr`] looks up a key and [`trf`] fills its placeholders. Missing translations
//! fall back to the English catalog; an unknown key is returned unchanged.
//!
//! Texts kept in tables and translated later (palette entries, labels…) are
//! marked with [`key`] so the completeness test can find them.

mod en;
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

    /// The translation table, indexed by stable message keys.
    pub fn table(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Lang::En => en::TABLE,
            Lang::Fr => fr::TABLE,
            Lang::Es => es::TABLE,
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

/// Marks a stable message key translated later with [`tr`]. Identity.
pub const fn key(message: &'static str) -> &'static str {
    message
}

/// Looks up a message key in the current language.
pub fn tr(message: &'static str) -> &'static str {
    translate(lang(), message)
}

fn translations(lang: Lang) -> &'static HashMap<&'static str, &'static str> {
    static EN: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static FR: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static ES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    let cache = match lang {
        Lang::En => &EN,
        Lang::Fr => &FR,
        Lang::Es => &ES,
    };
    cache.get_or_init(|| lang.table().iter().copied().collect())
}

fn resolve(
    message: &'static str,
    preferred: &HashMap<&'static str, &'static str>,
    english: &HashMap<&'static str, &'static str>,
) -> &'static str {
    preferred.get(message).or_else(|| english.get(message)).copied().unwrap_or(message)
}

/// Looks up a stable message key, with English as the fallback language.
pub fn translate(lang: Lang, message: &'static str) -> &'static str {
    resolve(message, translations(lang), translations(Lang::En))
}

/// Looks up a message key, then fills its placeholders: `{}` takes the next
/// argument, `{0}`, `{1}`… a given one (translations may reorder them).
pub fn trf(message: &'static str, args: &[&dyn Display]) -> String {
    fill(tr(message), args)
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
    fn fill_placeholders() {
        assert_eq!(fill("{} of {}", &[&1, &2]), "1 of 2");
        assert_eq!(fill("{1} before {0}", &[&"a", &"b"]), "b before a");
        assert_eq!(fill("set {a} {}", &[&3]), "set {a} 3");
        assert_eq!(placeholders("{1} x {0}"), vec![0, 1]);
    }

    #[test]
    fn fallback_and_lookup() {
        assert_eq!(translate(Lang::En, "file.save"), "Save");
        assert_eq!(translate(Lang::Fr, "file.save"), "Enregistrer");
        assert_eq!(translate(Lang::Es, "file.save"), "Guardar");
        for lang in Lang::ALL {
            assert_eq!(translate(lang, "unknown.message"), "unknown.message");
        }
        assert_eq!(
            fill(translate(Lang::Es, "file.open_failed"), &[&"test.opx", &"error"]),
            "No se puede abrir test.opx: error"
        );
    }

    #[test]
    fn english_copy_changes_without_changing_keys_or_translations() {
        let mut english = HashMap::from([("file.save", "Save")]);
        let french = HashMap::from([("file.save", "Enregistrer")]);
        let missing = HashMap::new();
        assert_eq!(resolve("file.save", &missing, &english), "Save");
        english.insert("file.save", "Save document");
        assert_eq!(resolve("file.save", &english, &english), "Save document");
        assert_eq!(resolve("file.save", &french, &english), "Enregistrer");
        assert_eq!(resolve("file.save", &missing, &english), "Save document");
        assert_eq!(resolve("unknown.message", &french, &english), "unknown.message");
    }

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
    fn tables_are_consistent() {
        let english: HashMap<_, _> = Lang::En.table().iter().copied().collect();
        for lang in Lang::ALL {
            let table = lang.table();
            let mut seen = std::collections::HashSet::new();
            for (message, t) in table {
                assert!(seen.insert(*message), "{lang:?}: duplicate entry {message:?}");
                assert!(message.contains('.'), "{lang:?}: key needs a namespace: {message:?}");
                assert!(
                    message
                        .split('.')
                        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')),
                    "{lang:?}: invalid key {message:?}"
                );
                let en = english.get(message).unwrap_or_else(|| panic!("{lang:?}: unknown key {message:?}"));
                assert!(!t.is_empty(), "{lang:?}: empty translation of {message:?}");
                assert_eq!(
                    placeholders(en),
                    placeholders(t),
                    "{lang:?}: placeholders differ for {message:?}: {en:?} -> {t:?}"
                );
                assert_eq!(
                    en.matches('\n').count(),
                    t.matches('\n').count(),
                    "{lang:?}: line count differs for {message:?}"
                );
            }
            assert_eq!(seen.len(), english.len(), "{lang:?}: missing catalog keys");
        }
    }
}
