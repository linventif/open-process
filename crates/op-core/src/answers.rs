//! Checking hand-written answers of the Alpha algorithm exercises.
//!
//! Accepts the usual notations of the course: `{A,B,C}`, `({A},{B,E})`,
//! `(A,BE)`, `P(A,BE)`, `p({A},{B,E})`, `iw`/`ow`, `A>B`, `A→B`, `A->B`,
//! `B||C`, `A#D`, arcs `(iw,A), (A,P(A,B))`…

use crate::i18n::trf;
use crate::mining::{AlphaResult, Pair, Relation};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Correct,
    Wrong {
        missing: Vec<String>,
        extra: Vec<String>,
    },
    /// The answer could not be read.
    Invalid(String),
    Empty,
}

impl Verdict {
    pub fn is_correct(&self) -> bool {
        matches!(self, Verdict::Correct)
    }
}

/// The 7 answer fields of the algorithm (step 8 is just the triple).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    Tw,
    Ti,
    To,
    Xw,
    Yw,
    Pw,
    Fw,
}

impl Step {
    pub const ALL: [Step; 7] = [Step::Tw, Step::Ti, Step::To, Step::Xw, Step::Yw, Step::Pw, Step::Fw];

    pub fn key(self) -> &'static str {
        match self {
            Step::Tw => "TW",
            Step::Ti => "TI",
            Step::To => "TO",
            Step::Xw => "XW",
            Step::Yw => "YW",
            Step::Pw => "PW",
            Step::Fw => "FW",
        }
    }
}

/// Activity names, with a "single letter" mode where `BE` means `B, E`.
pub struct Vocabulary {
    pub activities: Vec<String>,
    pub single_letter: bool,
}

impl Vocabulary {
    pub fn of(r: &AlphaResult) -> Self {
        let activities = r.footprint.activities.clone();
        let single_letter = activities.iter().all(|a| a.chars().count() == 1);
        Self { activities, single_letter }
    }

    fn find(&self, tok: &str) -> Option<String> {
        self.activities
            .iter()
            .find(|a| a.as_str() == tok || a.eq_ignore_ascii_case(tok))
            .cloned()
    }

    /// Reads `A`, `BE` (single-letter mode), `{A, B}`, `A,B`.
    pub fn parse_set(&self, s: &str) -> Result<BTreeSet<String>, String> {
        let inner = s.trim().trim_start_matches('{').trim_end_matches('}');
        let mut out = BTreeSet::new();
        for tok in inner
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter(|t| !t.is_empty())
        {
            if let Some(a) = self.find(tok) {
                out.insert(a);
            } else if self.single_letter && tok.chars().all(|c| self.find(&c.to_string()).is_some()) {
                out.extend(tok.chars().filter_map(|c| self.find(&c.to_string())));
            } else {
                return Err(trf("mining.unknown_activity", &[&tok]));
            }
        }
        Ok(out)
    }

    fn set_str(&self, s: &BTreeSet<String>) -> String {
        let v: Vec<&str> = s.iter().map(String::as_str).collect();
        format!("{{{}}}", v.join(","))
    }

    fn pair_str(&self, a: &BTreeSet<String>, b: &BTreeSet<String>) -> String {
        format!("({},{})", self.set_str(a), self.set_str(b))
    }
}

/// Splits `s` at commas that are not nested in () or {}.
fn split_top(s: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in s.chars() {
        match c {
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            ',' | ';' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out.into_iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
}

/// Removes one pair of surrounding braces, if the whole string is wrapped.
fn unwrap_outer(s: &str, open: char, close: char) -> &str {
    let t = s.trim();
    if t.starts_with(open) && t.ends_with(close) {
        // make sure the first bracket closes at the very end
        let mut depth = 0;
        for (i, c) in t.char_indices() {
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 && i != t.len() - close.len_utf8() {
                    return t;
                }
            }
        }
        return &t[open.len_utf8()..t.len() - close.len_utf8()];
    }
    t
}

/// `(A,B)` / `({A},{B,E})` → (A, B) sets.
fn parse_pair(v: &Vocabulary, s: &str) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let inner = unwrap_outer(s, '(', ')');
    let parts = split_top(inner);
    if parts.len() != 2 {
        return Err(trf("mining.not_pair_b", &[&s]));
    }
    Ok((v.parse_set(&parts[0])?, v.parse_set(&parts[1])?))
}

fn canon_place(v: &Vocabulary, s: &str) -> Result<String, String> {
    let t = s.trim();
    let low = t.to_lowercase();
    if ["iw", "i", "i_w", "start", "source"].contains(&low.as_str()) {
        return Ok("iw".into());
    }
    if ["ow", "o", "o_w", "end", "sink"].contains(&low.as_str()) {
        return Ok("ow".into());
    }
    let rest = t.strip_prefix(['P', 'p']).ok_or_else(|| trf("mining.not_place", &[&t]))?;
    let rest = rest.trim_start_matches(['_', ' ']);
    let rest = if rest.starts_with('{') && !rest.starts_with("{{") && !rest.contains('(') {
        // P_{A,BE}
        unwrap_outer(rest, '{', '}').to_string()
    } else {
        unwrap_outer(rest, '(', ')').to_string()
    };
    let (a, b) = parse_pair(v, &format!("({rest})"))?;
    Ok(format!("P{}", v.pair_str(&a, &b)))
}

fn diff(answer: BTreeSet<String>, expected: BTreeSet<String>) -> Verdict {
    if answer == expected {
        return Verdict::Correct;
    }
    Verdict::Wrong {
        missing: expected.difference(&answer).cloned().collect(),
        extra: answer.difference(&expected).cloned().collect(),
    }
}

fn items(s: &str) -> Vec<String> {
    split_top(unwrap_outer(s, '{', '}'))
}

/// Expected canonical items of a step.
pub fn expected(r: &AlphaResult, step: Step) -> BTreeSet<String> {
    let v = Vocabulary::of(r);
    let names = |s: &[usize]| -> BTreeSet<String> { s.iter().map(|&a| r.footprint.activities[a].clone()).collect() };
    let pair = |p: &Pair| v.pair_str(&names(&p.0), &names(&p.1));
    match step {
        Step::Tw => names(&r.t_w),
        Step::Ti => names(&r.t_i),
        Step::To => names(&r.t_o),
        Step::Xw => r.x_w.iter().map(pair).collect(),
        Step::Yw => r.y_w.iter().map(pair).collect(),
        Step::Pw => r
            .y_w
            .iter()
            .map(|p| format!("P{}", pair(p)))
            .chain(["iw".into(), "ow".into()])
            .collect(),
        Step::Fw => r
            .flow_relation()
            .into_iter()
            .map(|(a, b)| {
                let c = |x: &str| canon_place(&v, x).unwrap_or_else(|_| x.to_string());
                format!("({},{})", c(&a), c(&b))
            })
            .collect(),
    }
}

/// Parses an answer into canonical items.
pub fn parse(r: &AlphaResult, step: Step, answer: &str) -> Result<BTreeSet<String>, String> {
    let v = Vocabulary::of(r);
    match step {
        Step::Tw | Step::Ti | Step::To => v.parse_set(answer),
        Step::Xw | Step::Yw => items(answer)
            .iter()
            .map(|p| parse_pair(&v, p).map(|(a, b)| v.pair_str(&a, &b)))
            .collect(),
        Step::Pw => items(answer).iter().map(|p| canon_place(&v, p)).collect(),
        Step::Fw => items(answer)
            .iter()
            .map(|arc| {
                let inner = unwrap_outer(arc, '(', ')');
                let parts = split_top(inner);
                if parts.len() != 2 {
                    return Err(trf("mining.not_arc_x_y", &[&arc]));
                }
                let node = |x: &str| -> Result<String, String> {
                    match v.find(x.trim()) {
                        Some(a) => Ok(a),
                        None => canon_place(&v, x),
                    }
                };
                Ok(format!("({},{})", node(&parts[0])?, node(&parts[1])?))
            })
            .collect(),
    }
}

pub fn check(r: &AlphaResult, step: Step, answer: &str) -> Verdict {
    if answer.trim().is_empty() {
        return Verdict::Empty;
    }
    match parse(r, step, answer) {
        Ok(a) => diff(a, expected(r, step)),
        Err(e) => Verdict::Invalid(e),
    }
}

/// The model answer, in the course notation.
pub fn solution(r: &AlphaResult, step: Step) -> String {
    let pairs = |v: &[Pair]| v.iter().map(|p| r.pair(p)).collect::<Vec<_>>().join(", ");
    match step {
        Step::Tw => r.set(&r.t_w),
        Step::Ti => r.set(&r.t_i),
        Step::To => r.set(&r.t_o),
        Step::Xw => format!("{{{}}}", pairs(&r.x_w)),
        Step::Yw => format!("{{{}}}", pairs(&r.y_w)),
        Step::Pw => {
            let mut p: Vec<String> = r.y_w.iter().map(|p| r.place_name(p)).collect();
            p.extend(["iw".into(), "ow".into()]);
            format!("{{{}}}", p.join(", "))
        }
        Step::Fw => format!(
            "{{{}}}",
            r.flow_relation()
                .iter()
                .map(|(a, b)| format!("({a},{b})"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

// ---------------------------------------------------------------------------
// Relations written as lists ("A>B, A>C" / "A→B" / "B||C" / "A#D")

/// Which list of relations is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationList {
    Succession,
    Causality,
    Parallel,
    Independence,
}

impl RelationList {
    pub const ALL: [RelationList; 4] = [
        RelationList::Succession,
        RelationList::Causality,
        RelationList::Parallel,
        RelationList::Independence,
    ];

    pub fn key(self) -> &'static str {
        match self {
            RelationList::Succession => "succ",
            RelationList::Causality => "caus",
            RelationList::Parallel => "par",
            RelationList::Independence => "ind",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            RelationList::Succession => ">",
            RelationList::Causality => "→",
            RelationList::Parallel => "||",
            RelationList::Independence => "#",
        }
    }

    /// Symmetric relations are compared without order (B||C = C||B).
    fn symmetric(self) -> bool {
        matches!(self, RelationList::Parallel | RelationList::Independence)
    }
}

fn rel_expected(r: &AlphaResult, list: RelationList) -> BTreeSet<String> {
    let f = &r.footprint;
    let n = |a: usize| f.activities[a].clone();
    let pairs: Vec<(usize, usize)> = match list {
        RelationList::Succession => f.successions(),
        RelationList::Causality => f.pairs(Relation::Causal),
        RelationList::Parallel => f.pairs(Relation::Parallel),
        RelationList::Independence => f.pairs(Relation::Independent),
    };
    pairs.into_iter().map(|(a, b)| canon_rel(list, &n(a), &n(b))).collect()
}

fn canon_rel(list: RelationList, a: &str, b: &str) -> String {
    let (a, b) = if list.symmetric() && b < a { (b, a) } else { (a, b) };
    format!("{a}{}{b}", list.symbol())
}

pub fn check_relations(r: &AlphaResult, list: RelationList, answer: &str) -> Verdict {
    if answer.trim().is_empty() {
        return Verdict::Empty;
    }
    let v = Vocabulary::of(r);
    let syms: &[&str] = match list {
        RelationList::Succession => &[">"],
        RelationList::Causality => &["→", "->", "=>"],
        RelationList::Parallel => &["||", "∥", "//"],
        RelationList::Independence => &["#"],
    };
    let mut got = BTreeSet::new();
    for tok in answer.split([',', ';', '\n']).map(str::trim).filter(|t| !t.is_empty()) {
        let Some((sym, idx)) = syms.iter().find_map(|s| tok.find(s).map(|i| (*s, i))) else {
            return Verdict::Invalid(trf("mining.expected_x_y", &[&tok, &list.symbol()]));
        };
        let (a, b) = (tok[..idx].trim(), tok[idx + sym.len()..].trim());
        let (Some(a), Some(b)) = (v.find(a), v.find(b)) else {
            return Verdict::Invalid(trf("mining.unknown_activity_in_expression", &[&tok]));
        };
        got.insert(canon_rel(list, &a, &b));
    }
    diff(got, rel_expected(r, list))
}

pub fn relations_solution(r: &AlphaResult, list: RelationList) -> String {
    let f = &r.footprint;
    let n = |a: usize| f.activities[a].as_str();
    let pairs: Vec<(usize, usize)> = match list {
        RelationList::Succession => f.successions(),
        RelationList::Causality => f.pairs(Relation::Causal),
        RelationList::Parallel => f.pairs(Relation::Parallel),
        RelationList::Independence => f.pairs(Relation::Independent),
    };
    pairs
        .iter()
        .map(|&(a, b)| format!("{}{}{}", n(a), list.symbol(), n(b)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Builds the student's net from their TI, TO and YW answers (steps 6–8).
pub fn student_net(r: &AlphaResult, ti: &str, to: &str, yw: &str) -> Result<crate::petri::PetriNet, String> {
    let v = Vocabulary::of(r);
    let idx = |name: &String| r.footprint.activities.iter().position(|a| a == name).unwrap();
    let to_idx = |s: BTreeSet<String>| -> Vec<usize> { s.iter().map(idx).collect() };
    let t_i = to_idx(v.parse_set(ti).map_err(|e| format!("TI: {e}"))?);
    let t_o = to_idx(v.parse_set(to).map_err(|e| format!("TO: {e}"))?);
    let mut y_w = Vec::new();
    for p in items(yw) {
        let (a, b) = parse_pair(&v, &p).map_err(|e| format!("YW: {e}"))?;
        y_w.push((to_idx(a), to_idx(b)));
    }
    let mut mine = r.clone();
    mine.t_i = t_i;
    mine.t_o = t_o;
    mine.y_w = y_w;
    Ok(crate::mining::build_alpha_net(&mut mine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mining::{EventLog, alpha};

    fn ex2() -> AlphaResult {
        alpha(&EventLog::parse("ABCD ACBD AED").unwrap())
    }

    #[test]
    fn sets_and_pairs() {
        let r = ex2();
        assert_eq!(check(&r, Step::Ti, "{A}"), Verdict::Correct);
        assert_eq!(check(&r, Step::Tw, "A, B, C, D, E"), Verdict::Correct);
        assert!(matches!(check(&r, Step::To, "{D,E}"), Verdict::Wrong { .. }));
        assert_eq!(
            check(&r, Step::Yw, "{({A},{B,E}), ({A},{C,E}), ({B,E},{D}), ({C,E},{D})}"),
            Verdict::Correct
        );
        // compact notation and other order
        assert_eq!(check(&r, Step::Yw, "(A,CE), (A,BE), (BE,D), (CE,D)"), Verdict::Correct);
        match check(&r, Step::Yw, "(A,BE), (A,CE), (BE,D)") {
            Verdict::Wrong { missing, extra } => {
                assert_eq!(missing, vec!["({C,E},{D})".to_string()]);
                assert!(extra.is_empty());
            }
            v => panic!("{v:?}"),
        }
        assert!(matches!(check(&r, Step::Ti, "{Z}"), Verdict::Invalid(_)));
    }

    #[test]
    fn places_and_arcs() {
        let r = ex2();
        assert_eq!(
            check(&r, Step::Pw, "{P(A,BE), P(A,CE), P(BE,D), P(CE,D), iw, ow}"),
            Verdict::Correct
        );
        assert_eq!(
            check(&r, Step::Pw, "p({A},{B,E}), P_{A,CE}, P(BE,D), P(CE,D), i, o"),
            Verdict::Correct
        );
        assert_eq!(check(&r, Step::Fw, &solution(&r, Step::Fw)), Verdict::Correct);
        for s in Step::ALL {
            assert_eq!(check(&r, s, &solution(&r, s)), Verdict::Correct, "{s:?}");
        }
    }

    #[test]
    fn relation_lists() {
        let r = ex2();
        assert_eq!(check_relations(&r, RelationList::Parallel, "C||B"), Verdict::Correct);
        assert_eq!(
            check_relations(&r, RelationList::Causality, "A->B, A→C, A→E, B→D, C→D, E→D"),
            Verdict::Correct
        );
        assert!(matches!(
            check_relations(&r, RelationList::Independence, "A#D"),
            Verdict::Wrong { .. }
        ));
        for l in RelationList::ALL {
            assert_eq!(check_relations(&r, l, &relations_solution(&r, l)), Verdict::Correct, "{l:?}");
        }
    }

    #[test]
    fn net_from_student_answers() {
        let r = ex2();
        let net = student_net(&r, "{A}", "{D}", "(A,BE), (A,CE), (BE,D), (CE,D)").unwrap();
        assert_eq!(net.places.len(), 6);
        assert!(crate::mining::replay(&net, &["A".into(), "E".into(), "D".into()]).is_ok());
    }
}
