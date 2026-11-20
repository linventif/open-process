//! Process discovery: event logs, footprint (relation) matrix and the Alpha algorithm
//! (van der Aalst, Weijters & Maruster, 2004), with the notation used in class:
//! `>` direct succession, `→` causality, `||` parallelism, `#` independence,
//! and the 8 steps TW, TI, TO, XW, YW, PW, FW, α(W).

use crate::geom::Pos;
use crate::i18n::{tr, trf};
use crate::petri::{NodeRef, PetriNet};
use crate::xml;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

pub mod import;

// -------------------------------------------------------------------------
// Event log

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    /// One trace (case) = ordered list of activity names.
    pub traces: Vec<Vec<String>>,
}

/// CSV records with quoted separators, doubled quotes and embedded line breaks.
/// Invalid quoting is rejected rather than changing activity names silently.
fn csv_records(source: &str, delimiter: char) -> Result<Vec<Vec<String>>, String> {
    let mut chars = source.chars().peekable();
    let (mut records, mut row, mut field) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut was_quoted, mut closed) = (false, false, false);
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        if c == delimiter || c == '\n' || c == '\r' {
            let value = std::mem::take(&mut field);
            row.push(if was_quoted { value } else { value.trim().to_string() });
            was_quoted = false;
            closed = false;
            if c != delimiter {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                if row.len() > 1 || !row[0].is_empty() {
                    records.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
        } else if closed {
            if c != ' ' && c != '\t' {
                return Err(tr("mining.invalid_csv_quoting").into());
            }
        } else if c == '"' {
            if !field.trim().is_empty() {
                return Err(tr("mining.invalid_csv_quoting").into());
            }
            field.clear();
            quoted = true;
            was_quoted = true;
        } else {
            field.push(c);
        }
    }
    if quoted {
        return Err(tr("mining.invalid_csv_quoting").into());
    }
    if !field.is_empty() || !row.is_empty() || closed {
        row.push(if was_quoted { field } else { field.trim().to_string() });
        records.push(row);
    }
    Ok(records)
}

fn is_word(tok: &str) -> bool {
    tok.len() > 1 && tok.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

impl EventLog {
    /// Parses a log written in one of the following forms:
    ///
    /// * `Case 1, Task A` lines (cases may be interleaved), as in the course slides;
    /// * CSV with a header containing a case column and an activity/task column;
    /// * words of one-letter activities: `ABCD ACBD EF` (several per line allowed);
    /// * one trace per line, activities separated by spaces, commas, `;` or `->`.
    ///
    /// Empty lines and lines starting with `#` or `//` are ignored.
    pub fn parse(text: &str) -> Result<EventLog, String> {
        let trimmed = text.trim_start_matches('\u{feff}').trim_start();
        if trimmed.starts_with('<') {
            return Self::from_xes(trimmed);
        }
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
            .collect();
        if lines.is_empty() {
            return Err(tr("mining.log_empty").into());
        }

        if let Some(log) = Self::parse_csv(trimmed)? {
            return Ok(log);
        }

        // "Case 1, Task A" format (possibly several entries per line, as in multi-column tables)
        let case_entries = parse_case_lines(&lines);
        if !case_entries.is_empty() {
            let mut order: Vec<String> = Vec::new();
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            for (case, act) in case_entries {
                if !map.contains_key(&case) {
                    order.push(case.clone());
                }
                map.entry(case).or_default().push(act);
            }
            order.sort_by(|a, b| natural_cmp(a, b));
            return Ok(EventLog {
                traces: order.into_iter().map(|c| map.remove(&c).unwrap()).collect(),
            });
        }

        let split = |l: &str| -> Vec<String> {
            l.replace("->", " ")
                .replace('→', " ")
                .split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '|')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        };
        let tokens: Vec<Vec<String>> = lines.iter().map(|l| split(l)).collect();
        let all_words = tokens.iter().flatten().all(|t| is_word(t));
        let mut traces = Vec::new();
        if all_words {
            // Course style: every token is a word, every letter an activity.
            for t in tokens.iter().flatten() {
                traces.push(t.chars().map(|c| c.to_string()).collect());
            }
        } else {
            for t in tokens {
                if t.len() == 1 && is_word(&t[0]) {
                    traces.push(t[0].chars().map(|c| c.to_string()).collect());
                } else {
                    traces.push(t);
                }
            }
        }
        Ok(EventLog { traces })
    }

    fn parse_csv(mut source: &str) -> Result<Option<EventLog>, String> {
        while let Some(line) = source.lines().next() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') && !line.starts_with("//") {
                break;
            }
            source = source.split_once('\n').map(|(_, rest)| rest).unwrap_or("");
        }
        let header_line = source.lines().next().unwrap_or("");
        for delimiter in [',', ';', '\t'] {
            let Ok(header) = csv_records(header_line, delimiter) else {
                continue;
            };
            let Some(header) = header.first() else { continue };
            let header: Vec<_> = header.iter().map(|s| s.trim().to_lowercase()).collect();
            let normalized: Vec<_> = header.iter().map(|h| h.replace([' ', '_', '-'], "")).collect();
            let case_col = normalized.iter().position(|h| {
                matches!(
                    h.as_str(),
                    "case" | "caseid" | "id" | "instance" | "instanceid" | "case:concept:name"
                )
            });
            let activity_col = normalized.iter().position(|h| {
                matches!(
                    h.as_str(),
                    "activity" | "activityname" | "task" | "taskname" | "event" | "eventname" | "concept:name"
                )
            });
            let (Some(case_col), Some(activity_col)) = (case_col, activity_col) else {
                continue;
            };
            if case_col == activity_col {
                continue;
            }
            let records = csv_records(source, delimiter)?;
            let mut order = Vec::new();
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            for (index, record) in records.into_iter().enumerate().skip(1) {
                if record.len() != header.len() || record[case_col].trim().is_empty() || record[activity_col].trim().is_empty() {
                    return Err(trf("mining.invalid_csv_record", &[&(index + 1)]));
                }
                let case = record[case_col].trim().to_string();
                if !map.contains_key(&case) {
                    order.push(case.clone());
                }
                map.entry(case).or_default().push(record[activity_col].clone());
            }
            return Ok(Some(EventLog {
                traces: order.into_iter().map(|case| map.remove(&case).unwrap()).collect(),
            }));
        }
        Ok(None)
    }

    /// IEEE XES (as produced by ProM): `<log><trace><event><string key="concept:name" value="A"/>`.
    pub fn from_xes(src: &str) -> Result<EventLog, String> {
        let root = xml::parse(src)?;
        if root.name != "log" {
            return Err(trf("mining.not_xes_log_root_element", &[&root.name]));
        }
        let name_of = |e: &xml::Element| {
            e.children
                .iter()
                .find(|c| c.attr("key") == Some("concept:name"))
                .and_then(|c| c.attr("value"))
                .map(str::to_string)
        };
        let traces = root
            .children_named("trace")
            .map(|t| t.children_named("event").filter_map(name_of).collect::<Vec<_>>())
            .filter(|t| !t.is_empty())
            .collect();
        Ok(EventLog { traces })
    }

    /// Distinct traces with their frequency, in order of first appearance.
    pub fn variants(&self) -> Vec<(Vec<String>, usize)> {
        let mut out: Vec<(Vec<String>, usize)> = Vec::new();
        for t in &self.traces {
            match out.iter_mut().find(|(v, _)| v == t) {
                Some((_, n)) => *n += 1,
                None => out.push((t.clone(), 1)),
            }
        }
        out
    }

    /// Activities, in natural (alphabetical) order.
    pub fn activities(&self) -> Vec<String> {
        let mut set: Vec<String> = self.traces.iter().flatten().cloned().collect::<HashSet<_>>().into_iter().collect();
        set.sort_by(|a, b| natural_cmp(a, b));
        set
    }

    /// True if all activity names are a single character (compact notation `ABCD`).
    pub fn single_letter(&self) -> bool {
        self.traces.iter().flatten().all(|a| a.chars().count() == 1)
    }

    pub fn format_trace(&self, t: &[String]) -> String {
        if self.single_letter() {
            t.concat()
        } else {
            format!("⟨{}⟩", t.join(", "))
        }
    }
}

/// Byte index of an ASCII, case-insensitive match (always a char boundary).
fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    let n = needle.len();
    hay.as_bytes().windows(n).position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Extracts `(case, activity)` pairs from "Case 1, Task A" lines.
fn parse_case_lines(lines: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut matched_lines = 0;
    for l in lines {
        // Several "Case x, Task y" groups may share a line (tables with columns).
        let mut found = false;
        let mut rest: &str = l;
        while let Some(i) = find_ci(rest, "case") {
            let s = rest[i + 4..].trim_start_matches([' ', ':', '#']);
            let case: String = s.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
            if case.is_empty() {
                break;
            }
            let after = s[case.len()..].trim_start_matches([' ', ',', ';', ':', '\t']);
            let act_part = ["task", "activity", "event"]
                .iter()
                .find_map(|k| {
                    after
                        .get(..k.len())
                        .filter(|p| p.eq_ignore_ascii_case(k))
                        .map(|_| after[k.len()..].trim_start_matches([' ', ':']))
                })
                .unwrap_or(after);
            let next_case = find_ci(act_part, "case").unwrap_or(act_part.len());
            let act = act_part[..next_case].trim().trim_end_matches([',', ';']).trim();
            if act.is_empty() {
                break;
            }
            out.push((case, act.to_string()));
            found = true;
            rest = &act_part[next_case..];
        }
        if found {
            matched_lines += 1;
        }
    }
    if matched_lines * 2 >= lines.len() { out } else { Vec::new() }
}

/// Natural ordering: "Task 2" < "Task 10".
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |s: &str| -> Vec<(String, u64)> {
        let mut out = Vec::new();
        let mut text = String::new();
        let mut num = String::new();
        for c in s.chars() {
            if c.is_ascii_digit() {
                num.push(c);
            } else {
                if !num.is_empty() {
                    out.push((std::mem::take(&mut text), num.parse().unwrap_or(0)));
                    num.clear();
                }
                text.push(c);
            }
        }
        out.push((text, num.parse().unwrap_or(0)));
        out
    };
    key(a).cmp(&key(b))
}

// -------------------------------------------------------------------------
// Footprint

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Relation {
    /// a → b
    Causal,
    /// a ← b
    Reverse,
    /// a || b
    Parallel,
    /// a # b
    Independent,
}

impl Relation {
    pub fn symbol(self) -> &'static str {
        match self {
            Relation::Causal => "→",
            Relation::Reverse => "←",
            Relation::Parallel => "||",
            Relation::Independent => "#",
        }
    }

    pub fn latex(self) -> &'static str {
        match self {
            Relation::Causal => "\\rightarrow",
            Relation::Reverse => "\\leftarrow",
            Relation::Parallel => "\\parallel",
            Relation::Independent => "\\#",
        }
    }
}

/// Relation matrix of a log ("footprint").
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Footprint {
    pub activities: Vec<String>,
    /// `succ[a][b]`: a > b (a directly followed by b in some trace).
    pub succ: Vec<Vec<bool>>,
    pub rel: Vec<Vec<Relation>>,
}

impl Footprint {
    pub fn from_log(log: &EventLog) -> Footprint {
        let activities = log.activities();
        let index: HashMap<&str, usize> = activities.iter().enumerate().map(|(i, a)| (a.as_str(), i)).collect();
        let n = activities.len();
        let mut succ = vec![vec![false; n]; n];
        for t in &log.traces {
            for w in t.windows(2) {
                succ[index[w[0].as_str()]][index[w[1].as_str()]] = true;
            }
        }
        let rel = (0..n)
            .map(|a| {
                (0..n)
                    .map(|b| match (succ[a][b], succ[b][a]) {
                        (true, false) => Relation::Causal,
                        (false, true) => Relation::Reverse,
                        (true, true) => Relation::Parallel,
                        (false, false) => Relation::Independent,
                    })
                    .collect()
            })
            .collect();
        Footprint { activities, succ, rel }
    }

    pub fn len(&self) -> usize {
        self.activities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.activities.is_empty()
    }

    pub fn causal(&self, a: usize, b: usize) -> bool {
        self.rel[a][b] == Relation::Causal
    }

    pub fn independent(&self, a: usize, b: usize) -> bool {
        self.rel[a][b] == Relation::Independent
    }

    /// Pairs in matrix order; symmetric pairs occur once, excluding independent diagonals.
    pub fn pairs(&self, r: Relation) -> Vec<(usize, usize)> {
        let n = self.len();
        let sym = matches!(r, Relation::Parallel | Relation::Independent);
        let mut out = Vec::new();
        for (a, row) in self.rel.iter().enumerate().take(n) {
            for (b, &relation) in row.iter().enumerate().take(n) {
                if relation != r {
                    continue;
                }
                if (sym && a > b) || (r == Relation::Independent && a == b) {
                    continue;
                }
                out.push((a, b));
            }
        }
        out
    }

    pub fn successions(&self) -> Vec<(usize, usize)> {
        let n = self.len();
        (0..n)
            .flat_map(|a| (0..n).map(move |b| (a, b)))
            .filter(|&(a, b)| self.succ[a][b])
            .collect()
    }

    /// Cell content as shown in class (empty diagonal).
    pub fn cell(&self, a: usize, b: usize) -> &'static str {
        if a == b && self.rel[a][b] == Relation::Independent {
            ""
        } else {
            self.rel[a][b].symbol()
        }
    }

    pub fn to_latex(&self) -> String {
        let n = self.len();
        let mut s = format!("\\begin{{tabular}}{{|c|{}}}\n\\hline\n", "c|".repeat(n));
        s += &format!(" & {} \\\\\n\\hline\n", self.activities.join(" & "));
        for a in 0..n {
            let cells: Vec<String> = (0..n)
                .map(|b| {
                    if self.cell(a, b).is_empty() {
                        String::new()
                    } else {
                        format!("${}$", self.rel[a][b].latex())
                    }
                })
                .collect();
            s += &format!("{} & {} \\\\\n\\hline\n", self.activities[a], cells.join(" & "));
        }
        s + "\\end{tabular}\n"
    }

    /// CSV (one row per activity, the relation symbols as values).
    pub fn to_csv(&self) -> String {
        let esc = |v: &str| {
            if v.contains([',', '"']) {
                format!("\"{}\"", v.replace('"', "\"\""))
            } else {
                v.to_string()
            }
        };
        let mut out = format!(",{}\n", self.activities.iter().map(|a| esc(a)).collect::<Vec<_>>().join(","));
        for (a, name) in self.activities.iter().enumerate() {
            let row: Vec<String> = (0..self.len()).map(|b| esc(self.cell(a, b))).collect();
            out += &format!("{},{}\n", esc(name), row.join(","));
        }
        out
    }

    pub fn to_text(&self) -> String {
        let w = self.activities.iter().map(|a| a.chars().count()).max().unwrap_or(1).max(2) + 2;
        let pad = |s: &str| format!("{s:^w$}");
        let mut out = format!(
            "{}|{}\n",
            pad(""),
            self.activities.iter().map(|a| pad(a)).collect::<Vec<_>>().join("|")
        );
        for (a, name) in self.activities.iter().enumerate() {
            let row: Vec<String> = (0..self.len()).map(|b| pad(self.cell(a, b))).collect();
            out += &format!("{}|{}\n", pad(name), row.join("|"));
        }
        out
    }
}

// -------------------------------------------------------------------------
// Alpha algorithm

pub type Pair = (Vec<usize>, Vec<usize>);

/// Optional common-boundary preprocessing for application discovery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryOptions {
    /// Add common case boundaries so parallel initial/final activities can synchronize.
    #[serde(default)]
    pub normalize_boundaries: bool,
}

/// Recorded routing placeholders are whole names, never a substring of a task.
/// For example, `Vide_2` is a marker; `Empty basket` and `Vider oven` are tasks.
pub fn is_routing_marker(activity: &str) -> bool {
    let name = activity.trim().to_ascii_lowercase();
    let Some(suffix) = name.strip_prefix("vide").or_else(|| name.strip_prefix("empty")) else {
        return false;
    };
    if suffix.is_empty() {
        return true;
    }
    let number = suffix.trim_start_matches(|c: char| c == '_' || c == '-' || c.is_whitespace());
    !number.is_empty() && number.bytes().all(|digit| digit.is_ascii_digit())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlphaResult {
    pub footprint: Footprint,
    /// Step 1: all activities.
    pub t_w: Vec<usize>,
    /// Step 2: initial activities.
    pub t_i: Vec<usize>,
    /// Step 3: final activities.
    pub t_o: Vec<usize>,
    /// Step 4: causal relations between sets.
    pub x_w: Vec<Pair>,
    /// True if X_W was too large and has been truncated.
    pub x_truncated: bool,
    /// Step 5: maximal pairs.
    pub y_w: Vec<Pair>,
    /// Steps 6–8: the discovered workflow net (places p(A,B), iw, ow).
    pub net: PetriNet,
    /// Petri net place id of each Y_W pair (same order as `y_w`).
    pub place_ids: Vec<u32>,
    /// Generated boundary activity names; their transitions are silent in the net.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_activities: Option<[String; 2]>,
    /// Recorded routing markers projected to unnamed silent transitions after Alpha.
    /// The original names remain in the source log and construction footprint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projected_activities: Vec<String>,
}

const MAX_X: usize = 20_000;

impl AlphaResult {
    pub fn name(&self, a: usize) -> &str {
        &self.footprint.activities[a]
    }

    /// `{A,B}` (or `{a, b}` for long names).
    pub fn set(&self, s: &[usize]) -> String {
        let names: Vec<&str> = s.iter().map(|&a| self.name(a)).collect();
        if self.single_letter() {
            format!("{{{}}}", names.join(","))
        } else {
            format!("{{{}}}", names.join(", "))
        }
    }

    fn single_letter(&self) -> bool {
        self.footprint.activities.iter().all(|a| a.chars().count() == 1)
    }

    pub fn pair(&self, p: &Pair) -> String {
        format!("({},{})", self.set(&p.0), self.set(&p.1))
    }

    /// Place name as in class: `P(A,BE)`.
    pub fn place_name(&self, p: &Pair) -> String {
        if self.single_letter() {
            let a: String = p.0.iter().map(|&x| self.name(x)).collect();
            let b: String = p.1.iter().map(|&x| self.name(x)).collect();
            format!("P({a},{b})")
        } else {
            format!("P({},{})", self.set(&p.0), self.set(&p.1))
        }
    }

    /// Project observed traces explicitly, without changing replay of raw Alpha
    /// nets or ordinary tasks whose names contain words such as "Empty".
    pub fn project_trace(&self, trace: &[String]) -> Vec<String> {
        trace
            .iter()
            .filter(|activity| {
                !self.projected_activities.contains(*activity)
                    || self
                        .net
                        .transitions
                        .iter()
                        .any(|transition| !transition.silent && transition.name.as_str() == activity.as_str())
            })
            .cloned()
            .collect()
    }

    /// Label a generated Petri place using silent routing symbols. The original
    /// Alpha construction remains available through `place_name`.
    pub fn net_place_name(&self, pair: &Pair) -> String {
        if self.projected_activities.is_empty() {
            return self.place_name(pair);
        }
        let set = |indices: &[usize]| {
            let labels: Vec<_> = indices
                .iter()
                .map(|&index| {
                    let name = self.name(index);
                    if self.projected_activities.iter().any(|projected| projected == name) {
                        "τ"
                    } else {
                        name
                    }
                })
                .collect();
            format!("{{{}}}", labels.join(", "))
        };
        format!("P({},{})", set(&pair.0), set(&pair.1))
    }

    fn apply_recorded_activity_projection(&mut self) {
        for transition in &mut self.net.transitions {
            if self.projected_activities.contains(&transition.name) {
                transition.name.clear();
                transition.silent = true;
            }
        }
        let place_names: Vec<_> = self.y_w.iter().map(|pair| self.net_place_name(pair)).collect();
        for (&id, name) in self.place_ids.iter().zip(place_names) {
            if let Some(place) = self.net.place_mut(id) {
                place.name = name;
            }
        }
    }

    /// Arcs of step 7 (F_W) as text pairs.
    pub fn flow_relation(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for &t in &self.t_i {
            out.push(("iw".to_string(), self.name(t).to_string()));
        }
        for p in &self.y_w {
            let pn = self.place_name(p);
            for &a in &p.0 {
                out.push((self.name(a).to_string(), pn.clone()));
            }
            for &b in &p.1 {
                out.push((pn.clone(), self.name(b).to_string()));
            }
        }
        for &t in &self.t_o {
            out.push((self.name(t).to_string(), "ow".to_string()));
        }
        out
    }

    /// Full step-by-step write-up (plain text), as expected in exercises.
    pub fn report(&self, log: &EventLog) -> String {
        let replays: Vec<_> = log
            .variants()
            .iter()
            .map(|(trace, _)| replay(&self.net, &self.project_trace(trace)))
            .collect();
        self.report_with_replays(log, &replays)
    }

    /// Format a write-up using cached outcomes in `log.variants()` order.
    /// Missing outcomes remain pending; formatting never starts additional replay.
    pub fn report_with_replays(&self, log: &EventLog, replays: &[Result<(), String>]) -> String {
        let f = &self.footprint;
        let n = |a: usize| self.name(a).to_string();
        let mut s = String::new();
        if self.boundary_activities.is_some() {
            s += tr("mining.report.normalized_boundaries");
            s.push('\n');
        }
        if !self.projected_activities.is_empty() {
            s += tr("mining.report.projected_markers");
            s.push('\n');
        }
        s += tr("mining.report.traces");
        for (t, k) in log.variants() {
            s += &format!(
                "  {}{}\n",
                log.format_trace(&t),
                if k > 1 { format!("  (×{k})") } else { String::new() }
            );
        }
        let list = |pairs: Vec<(usize, usize)>, sym: &str| -> String {
            pairs
                .iter()
                .map(|&(a, b)| format!("{}{sym}{}", n(a), n(b)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        s += &trf("mining.direct_successions", &[&(list(f.successions(), ">"))]);
        s += &trf("mining.report.causality", &[&(list(f.pairs(Relation::Causal), "→"))]);
        let par: Vec<(usize, usize)> = f
            .pairs(Relation::Parallel)
            .into_iter()
            .flat_map(|(a, b)| if a == b { vec![(a, b)] } else { vec![(a, b), (b, a)] })
            .collect();
        s += &trf("mining.parallelism", &[&(list(par, "||"))]);
        s += &trf("mining.independence", &[&(list(f.pairs(Relation::Independent), "#"))]);
        s += &trf("mining.report.relation_matrix", &[&f.to_text()]);
        s += &format!("1. TW = {}\n", self.set(&self.t_w));
        s += &format!("2. TI = {}\n", self.set(&self.t_i));
        s += &format!("3. TO = {}\n", self.set(&self.t_o));
        s += &format!(
            "4. XW = {{{}}}\n",
            self.x_w.iter().map(|p| self.pair(p)).collect::<Vec<_>>().join(", ")
        );
        s += &format!(
            "5. YW = {{{}}}\n",
            self.y_w.iter().map(|p| self.pair(p)).collect::<Vec<_>>().join(", ")
        );
        let mut places: Vec<String> = self.y_w.iter().map(|p| self.place_name(p)).collect();
        places.extend(["iw".into(), "ow".into()]);
        s += &format!("6. PW = {{{}}}\n", places.join(", "));
        s += &format!(
            "7. FW = {{{}}}\n",
            self.flow_relation()
                .iter()
                .map(|(a, b)| format!("({a},{b})"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        s += "8. α(W) = (PW, TW, FW)\n\n";
        s += tr("mining.report.word_check");
        for (index, (t, _)) in log.variants().into_iter().enumerate() {
            s += &format!(
                "  {} ? {}\n",
                log.format_trace(&t),
                match replays.get(index) {
                    Some(Ok(())) => "ok".to_string(),
                    Some(Err(e)) => trf("mining.not_ok", &[e]),
                    None => tr("mining.analysis_pending").to_string(),
                }
            );
        }
        s
    }
}

pub fn alpha(log: &EventLog) -> AlphaResult {
    alpha_bounded(log, MAX_X, &|| false)
}

/// Application discovery rejects incomplete results and bounds costly pair construction.
pub fn discover_cancellable(log: &EventLog, cancelled: &impl Fn() -> bool) -> Result<AlphaResult, String> {
    discover_with_options_cancellable(log, &DiscoveryOptions::default(), cancelled)
}

/// Discover a net with optional common boundaries, keeping the source log unchanged.
///
/// Generated activities remain visible in Alpha's construction and are marked silent
/// only in the resulting net. Recorded Vide/Empty routing markers are likewise
/// projected after construction, keeping the source and footprint unchanged.
/// Limits on source events exclude generated boundaries.
pub fn discover_with_options_cancellable(
    log: &EventLog,
    options: &DiscoveryOptions,
    cancelled: &impl Fn() -> bool,
) -> Result<AlphaResult, String> {
    if log.traces.iter().map(Vec::len).sum::<usize>() > 100_000 || log.activities().len() > 128 || cancelled() {
        return Err(tr("mining.analysis_limit").into());
    }
    let boundaries = options.normalize_boundaries.then(|| {
        let activities: HashSet<_> = log.activities().into_iter().collect();
        let unique = |base: &str| {
            let mut name = base.to_string();
            let mut suffix = 2;
            while activities.contains(&name) {
                name = format!("{base} ({suffix})");
                suffix += 1;
            }
            name
        };
        [unique("[start]"), unique("[end]")]
    });
    let normalized = boundaries.as_ref().map(|[start, end]| EventLog {
        traces: log
            .traces
            .iter()
            .map(|trace| {
                std::iter::once(start.clone())
                    .chain(trace.iter().cloned())
                    .chain(std::iter::once(end.clone()))
                    .collect()
            })
            .collect(),
    });
    let input = normalized.as_ref().unwrap_or(log);
    if input.activities().len() > 128 || cancelled() {
        return Err(tr("mining.analysis_limit").into());
    }
    let mut result = alpha_bounded(input, 2000, cancelled);
    if result.x_truncated || cancelled() {
        return Err(tr("mining.analysis_limit").into());
    }
    if let Some(boundaries) = boundaries {
        for transition in &mut result.net.transitions {
            transition.silent = boundaries.contains(&transition.name);
        }
        result.boundary_activities = Some(boundaries);
    }
    result.projected_activities = log
        .activities()
        .into_iter()
        .filter(|activity| is_routing_marker(activity))
        .collect();
    result.apply_recorded_activity_projection();
    result.net.check_storage()?;
    Ok(result)
}

fn alpha_bounded(log: &EventLog, pair_limit: usize, cancelled: &impl Fn() -> bool) -> AlphaResult {
    let fp = Footprint::from_log(log);
    let n = fp.len();
    let index: HashMap<&str, usize> = fp.activities.iter().enumerate().map(|(i, a)| (a.as_str(), i)).collect();
    let t_w: Vec<usize> = (0..n).collect();
    let mut t_i: Vec<usize> = log.traces.iter().filter_map(|t| t.first()).map(|a| index[a.as_str()]).collect();
    let mut t_o: Vec<usize> = log.traces.iter().filter_map(|t| t.last()).map(|a| index[a.as_str()]).collect();
    t_i.sort();
    t_i.dedup();
    t_o.sort();
    t_o.dedup();

    // Step 4: X_W — breadth-first growth from the singleton pairs ({a},{b}), a → b.
    // Valid pairs are closed under taking non-empty subsets, so every pair is reached.
    let valid_add_a = |a: &[usize], b: &[usize], x: usize| {
        !a.contains(&x) && fp.independent(x, x) && a.iter().all(|&y| fp.independent(x, y)) && b.iter().all(|&y| fp.causal(x, y))
    };
    let valid_add_b = |a: &[usize], b: &[usize], x: usize| {
        !b.contains(&x) && fp.independent(x, x) && b.iter().all(|&y| fp.independent(x, y)) && a.iter().all(|&y| fp.causal(y, x))
    };
    let mut seen: HashSet<Pair> = HashSet::new();
    let mut queue: VecDeque<Pair> = VecDeque::new();
    for (a, b) in fp.pairs(Relation::Causal) {
        if fp.independent(a, a) && fp.independent(b, b) {
            let p = (vec![a], vec![b]);
            if seen.insert(p.clone()) {
                queue.push_back(p);
            }
        }
    }
    let mut x_truncated = false;
    while let Some((a, b)) = queue.pop_front() {
        if seen.len() >= pair_limit || cancelled() {
            x_truncated = true;
            break;
        }
        for x in 0..n {
            if valid_add_a(&a, &b, x) {
                let mut a2 = a.clone();
                a2.push(x);
                a2.sort();
                let p = (a2, b.clone());
                if seen.insert(p.clone()) {
                    queue.push_back(p);
                }
            }
            if valid_add_b(&a, &b, x) {
                let mut b2 = b.clone();
                b2.push(x);
                b2.sort();
                let p = (a.clone(), b2);
                if seen.insert(p.clone()) {
                    queue.push_back(p);
                }
            }
        }
    }
    let mut x_w: Vec<Pair> = seen.into_iter().collect();
    x_w.sort_by(|p, q| (p.0.len() + p.1.len(), &p.0, &p.1).cmp(&(q.0.len() + q.1.len(), &q.0, &q.1)));

    // Charge a conservative upper bound for subset comparisons before doing them.
    let mut work = 20_000_000usize;
    let mut y_w = Vec::new();
    'maximal: for p in &x_w {
        if x_truncated {
            break;
        }
        if cancelled() {
            x_truncated = true;
            break;
        }
        let mut maximal = true;
        for q in &x_w {
            if p == q {
                continue;
            }
            let cost =
                p.0.len()
                    .saturating_mul(q.0.len())
                    .saturating_add(p.1.len().saturating_mul(q.1.len()));
            let Some(remaining) = work.checked_sub(cost.max(1)) else {
                x_truncated = true;
                break 'maximal;
            };
            work = remaining;
            if p.0.iter().all(|x| q.0.contains(x)) && p.1.iter().all(|x| q.1.contains(x)) {
                maximal = false;
                break;
            }
        }
        if maximal {
            y_w.push(p.clone());
        }
    }
    if y_w.iter().map(|(a, b)| a.len() + b.len()).sum::<usize>() > 10_000 {
        x_truncated = true;
    }

    let mut res = AlphaResult {
        footprint: fp,
        t_w,
        t_i,
        t_o,
        x_w,
        x_truncated,
        y_w,
        net: PetriNet::new("Alpha"),
        place_ids: Vec::new(),
        boundary_activities: None,
        projected_activities: Vec::new(),
    };
    if !res.x_truncated && !cancelled() {
        build_net(&mut res);
    }
    res
}

/// Steps 6–8 for the given TI, TO and YW (used for the student's own answers).
pub fn build_alpha_net(r: &mut AlphaResult) -> PetriNet {
    build_net(r);
    r.net.clone()
}

/// Steps 6–8 with an automatic left-to-right layout.
fn build_net(r: &mut AlphaResult) {
    let n = r.footprint.len();
    let mut net = PetriNet::new(tr("mining.discovered_process_alpha"));

    // Level of each activity: BFS distance from the initial activities.
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a_set, b_set) in &r.y_w {
        for &a in a_set {
            for &b in b_set {
                succ[a].push(b);
            }
        }
    }
    let mut level = vec![usize::MAX; n];
    let mut q: VecDeque<usize> = r.t_i.iter().copied().collect();
    for &t in &r.t_i {
        level[t] = 0;
    }
    while let Some(a) = q.pop_front() {
        for &b in &succ[a] {
            if level[b] == usize::MAX {
                level[b] = level[a] + 1;
                q.push_back(b);
            }
        }
    }
    for l in &mut level {
        if *l == usize::MAX {
            *l = 0;
        }
    }
    let mut columns: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (a, &l) in level.iter().enumerate() {
        columns.entry(l).or_default().push(a);
    }
    const DX: f32 = 200.0;
    const DY: f32 = 110.0;
    let mut tpos = vec![Pos::default(); n];
    for (lvl, acts) in &columns {
        let k = acts.len() as f32;
        for (i, &a) in acts.iter().enumerate() {
            tpos[a] = Pos::new(160.0 + *lvl as f32 * DX, 300.0 + (i as f32 - (k - 1.0) / 2.0) * DY);
        }
    }
    let mut tid = vec![0u32; n];
    for a in 0..n {
        let id = net.add_transition(tpos[a]);
        let transition = net.transition_mut(id).unwrap();
        transition.name = r.footprint.activities[a].clone();
        transition.silent = r.boundary_activities.as_ref().is_some_and(|names| names.contains(&transition.name));
        if r.projected_activities.contains(&transition.name) {
            transition.name.clear();
            transition.silent = true;
        }
        tid[a] = id;
    }
    let avg = |s: &[usize]| -> Pos {
        let k = s.len().max(1) as f32;
        Pos::new(
            s.iter().map(|&a| tpos[a].x).sum::<f32>() / k,
            s.iter().map(|&a| tpos[a].y).sum::<f32>() / k,
        )
    };
    let mut used: Vec<Pos> = Vec::new();
    let mut free_spot = |mut p: Pos| {
        let mut guard = 0;
        while used.iter().any(|u| u.dist(p) < 50.0) && guard < 20 {
            p.y += 55.0;
            guard += 1;
        }
        used.push(p);
        p
    };

    let iw_pos = free_spot(Pos::new(60.0, avg(&r.t_i).y));
    let iw = net.add_place(iw_pos, 1);
    net.place_mut(iw).unwrap().name = "iw".into();
    for &t in &r.t_i {
        let _ = net.add_arc(NodeRef::Place(iw), NodeRef::Transition(tid[t]));
    }
    let mut place_ids = Vec::new();
    for p in r.y_w.clone() {
        let (a, b) = (avg(&p.0), avg(&p.1));
        let mut pos = Pos::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        if b.x <= a.x {
            // loop back: put the place below
            pos = Pos::new(a.x.max(b.x) + DX / 2.0, a.y.max(b.y) + DY * 0.8);
        }
        let pid = net.add_place(free_spot(pos), 0);
        net.place_mut(pid).unwrap().name = r.net_place_name(&p);
        for &x in &p.0 {
            let _ = net.add_arc(NodeRef::Transition(tid[x]), NodeRef::Place(pid));
        }
        for &y in &p.1 {
            let _ = net.add_arc(NodeRef::Place(pid), NodeRef::Transition(tid[y]));
        }
        place_ids.push(pid);
    }
    let max_x = tpos.iter().map(|p| p.x).fold(0.0, f32::max);
    let ow_pos = free_spot(Pos::new(max_x + DX / 2.0, avg(&r.t_o).y));
    let ow = net.add_place(ow_pos, 0);
    net.place_mut(ow).unwrap().name = "ow".into();
    for &t in &r.t_o {
        let _ = net.add_arc(NodeRef::Transition(tid[t]), NodeRef::Place(ow));
    }
    r.net = net;
    r.place_ids = place_ids;
}

/// Replays a trace on a workflow net (token game, "vérification des mots"):
/// starting with one token in the source place, every activity must be enabled
/// in turn and the run must end with exactly one token in the sink place.
/// Silent transitions can fire before, between and after observed activities.
pub fn replay(net: &PetriNet, trace: &[String]) -> Result<(), String> {
    replay_cancellable(net, trace, &|| false)
}

/// Bounded replay explores alternative silent routes and equally named transitions.
/// Exceeding a search limit is an inconclusive error, never an accepted trace.
pub fn replay_cancellable(net: &PetriNet, trace: &[String], cancelled: &impl Fn() -> bool) -> Result<(), String> {
    let c = net.compile();
    let silent: Vec<_> = net
        .transitions
        .iter()
        .enumerate()
        .filter_map(|(i, t)| t.silent.then_some(i))
        .collect();
    let sources: Vec<usize> = (0..net.places.len())
        .filter(|&p| !net.arcs.iter().any(|a| a.to == NodeRef::Place(net.places[p].id)))
        .collect();
    let sinks: Vec<usize> = (0..net.places.len())
        .filter(|&p| !net.arcs.iter().any(|a| a.from == NodeRef::Place(net.places[p].id)))
        .collect();
    let mut m = vec![0u32; net.places.len()];
    for &s in &sources {
        m[s] = 1;
    }
    let mut states = vec![m];
    let mut budget = 1_000_000usize;
    for (k, act) in trace.iter().enumerate() {
        let cands: Vec<usize> = (0..net.transitions.len())
            .filter(|&t| !net.transitions[t].silent && &net.transitions[t].name == act)
            .collect();
        if cands.is_empty() {
            return Err(trf("mining.not_net", &[&act]));
        }
        let reachable = replay_silent_closure(&c, &silent, states, &mut budget, cancelled)?;
        let mut next = Vec::new();
        let mut seen = HashSet::new();
        for marking in reachable {
            for &t in &cands {
                replay_charge(&mut budget, cancelled)?;
                if let Some(fired) = c.fire(t, &marking)
                    && seen.insert(fired.clone())
                {
                    next.push(fired);
                    if next.len() > 5000 {
                        return Err(tr("mining.replay_limit").into());
                    }
                }
            }
        }
        if next.is_empty() {
            return Err(trf("mining.not_enabled_at_position", &[&act, &(k + 1)]));
        }
        states = next;
    }
    let states = replay_silent_closure(&c, &silent, states, &mut budget, cancelled)?;
    if states.iter().any(|m| {
        sinks.iter().all(|&s| m[s] <= 1)
            && sinks.iter().map(|&s| m[s]).sum::<u32>() == 1
            && m.iter().enumerate().all(|(p, &tokens)| tokens == 0 || sinks.contains(&p))
    }) {
        return Ok(());
    }
    // Keep the original teaching replay diagnostics when no route can finish.
    let m = &states[0];
    let ok_end = sinks.iter().all(|&s| m[s] <= 1) && sinks.iter().map(|&s| m[s]).sum::<u32>() == 1;
    let leftovers: Vec<&str> = (0..m.len())
        .filter(|&p| m[p] > 0 && !sinks.contains(&p))
        .map(|p| net.places[p].name.as_str())
        .collect();
    if !ok_end {
        let k: u32 = sinks.iter().map(|&s| m[s]).sum();
        let names: Vec<&str> = sinks.iter().map(|&s| net.places[s].name.as_str()).collect();
        return Err(trf("mining.ends_token_s_instead_exactly_1", &[&k, &(names.join(", "))]));
    }
    if !leftovers.is_empty() {
        return Err(trf("mining.tokens_left", &[&(leftovers.join(", "))]));
    }
    Ok(())
}

fn replay_charge(budget: &mut usize, cancelled: &impl Fn() -> bool) -> Result<(), String> {
    if cancelled() || *budget == 0 {
        return Err(tr("mining.replay_limit").into());
    }
    *budget -= 1;
    Ok(())
}

fn replay_silent_closure(
    net: &crate::petri::CompiledNet,
    silent: &[usize],
    seeds: Vec<Vec<u32>>,
    budget: &mut usize,
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<Vec<u32>>, String> {
    let mut seen: HashSet<Vec<u32>> = HashSet::new();
    let mut states = Vec::new();
    for marking in seeds {
        if seen.insert(marking.clone()) {
            states.push(marking);
        }
    }
    let mut index = 0;
    while index < states.len() {
        replay_charge(budget, cancelled)?;
        let marking = states[index].clone();
        for &transition in silent {
            replay_charge(budget, cancelled)?;
            if let Some(fired) = net.fire(transition, &marking)
                && seen.insert(fired.clone())
            {
                states.push(fired);
                if states.len() > 5000 {
                    return Err(tr("mining.replay_limit").into());
                }
            }
        }
        index += 1;
    }
    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprint_pairs_preserve_direction_symmetry_order_and_self_loops() {
        let log = EventLog::parse("AABC\nBAD\nCD\nDC").unwrap();
        let footprint = Footprint::from_log(&log);
        assert_eq!(footprint.activities, ["A", "B", "C", "D"]);
        assert_eq!(footprint.pairs(Relation::Causal), [(0, 3), (1, 2)]);
        assert_eq!(footprint.pairs(Relation::Reverse), [(2, 1), (3, 0)]);
        assert_eq!(footprint.pairs(Relation::Parallel), [(0, 0), (0, 1), (2, 3)]);
        assert_eq!(footprint.pairs(Relation::Independent), [(0, 2), (1, 3)]);
        let empty = Footprint::from_log(&EventLog::default());
        for relation in [Relation::Causal, Relation::Reverse, Relation::Parallel, Relation::Independent] {
            assert!(empty.pairs(relation).is_empty());
        }
    }

    #[test]
    fn discovery_refuses_cancelled_or_incomplete_results() {
        let log = EventLog::parse("ABCD\nACBD\nAED").unwrap();
        let result = discover_cancellable(&log, &|| false).unwrap();
        assert!(!result.x_truncated);
        assert_eq!(result.y_w, alpha(&log).y_w);
        assert!(discover_cancellable(&log, &|| true).is_err());
        let large = EventLog {
            traces: (0..129).map(|i| vec![format!("Activity {i}")]).collect(),
        };
        assert!(discover_cancellable(&large, &|| false).is_err());
        let many_pairs = EventLog {
            traces: (0..64)
                .flat_map(|a| (0..64).map(move |b| vec![format!("A{a}"), format!("B{b}")]))
                .collect(),
        };
        assert!(discover_cancellable(&many_pairs, &|| false).is_err());
    }

    #[test]
    fn normalized_discovery_synchronizes_parallel_ends_and_cancellation() {
        let log =
            EventLog::parse("case;activity\n1;Prepare\n1;Cash\n1;Give order\n2;Prepare\n2;Give order\n2;Cash\n3;Cancel order").unwrap();
        let original = log.clone();
        let teaching = discover_cancellable(&log, &|| false).unwrap();
        assert!(teaching.boundary_activities.is_none());
        assert!(replay(&teaching.net, &log.traces[0]).is_err());

        let mut normalized = discover_with_options_cancellable(
            &log,
            &DiscoveryOptions {
                normalize_boundaries: true,
            },
            &|| false,
        )
        .unwrap();
        assert_eq!(log, original);
        assert_eq!(normalized.net.transitions.iter().filter(|t| t.silent).count(), 2);
        for trace in &log.traces {
            assert_eq!(replay(&normalized.net, trace), Ok(()), "{trace:?}");
        }
        let invalid = vec!["Prepare".into(), "Cash".into()];
        assert!(replay(&normalized.net, &invalid).is_err());
        // Rebuilding from construction sets must preserve generated routing semantics.
        let rebuilt = build_alpha_net(&mut normalized);
        assert_eq!(rebuilt.transitions.iter().filter(|t| t.silent).count(), 2);
        assert!(replay(&rebuilt, &log.traces[0]).is_ok());
        assert!(normalized.report(&log).contains(tr("mining.report.normalized_boundaries")));
    }

    #[test]
    fn normalized_discovery_projects_recorded_routing_and_avoids_name_collisions() {
        let log = EventLog {
            traces: vec![vec!["[start]".into(), "Vide1".into(), "[end]".into()]],
        };
        let normalized = discover_with_options_cancellable(
            &log,
            &DiscoveryOptions {
                normalize_boundaries: true,
            },
            &|| false,
        )
        .unwrap();
        let boundaries = normalized.boundary_activities.as_ref().unwrap();
        assert!(boundaries.iter().all(|name| !log.traces[0].contains(name)));
        assert!(normalized.projected_activities.contains(&"Vide1".to_string()));
        assert!(normalized.net.transitions.iter().any(|t| t.name.is_empty() && t.silent));
        assert!(normalized.net.transitions.iter().any(|t| t.name == "[start]" && !t.silent));
        assert_eq!(replay(&normalized.net, &normalized.project_trace(&log.traces[0])), Ok(()));
    }

    #[test]
    fn normalized_discovery_synchronizes_parallel_case_starts() {
        let log = EventLog::parse("ABC\nBAC").unwrap();
        let teaching = discover_cancellable(&log, &|| false).unwrap();
        assert!(replay(&teaching.net, &log.traces[0]).is_err());
        let normalized = discover_with_options_cancellable(
            &log,
            &DiscoveryOptions {
                normalize_boundaries: true,
            },
            &|| false,
        )
        .unwrap();
        for trace in &log.traces {
            assert_eq!(replay(&normalized.net, trace), Ok(()));
        }
    }

    #[test]
    fn normalized_discovery_checks_limits_after_adding_boundaries() {
        let log = EventLog {
            traces: (0..127).map(|i| vec![format!("Activity {i}")]).collect(),
        };
        assert!(discover_cancellable(&log, &|| false).is_ok());
        assert!(
            discover_with_options_cancellable(
                &log,
                &DiscoveryOptions {
                    normalize_boundaries: true
                },
                &|| false
            )
            .is_err()
        );
        assert!(
            discover_with_options_cancellable(
                &EventLog::parse("AB").unwrap(),
                &DiscoveryOptions {
                    normalize_boundaries: true
                },
                &|| true
            )
            .is_err()
        );
    }

    #[test]
    fn discovery_configuration_and_result_read_legacy_json() {
        let options: DiscoveryOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(options, DiscoveryOptions::default());
        let result = alpha(&EventLog::parse("ABC").unwrap());
        let wire = serde_json::to_value(&result).unwrap();
        assert!(wire.get("boundary_activities").is_none());
        assert!(wire.get("projected_activities").is_none());
        let restored: AlphaResult = serde_json::from_value(wire).unwrap();
        assert!(restored.boundary_activities.is_none());
        assert!(restored.projected_activities.is_empty());
        assert_eq!(restored.net, result.net);
    }

    #[test]
    fn routing_marker_matching_is_whole_name_and_does_not_hide_business_tasks() {
        for name in ["Vide", "EMPTY", "vide1", "Empty_002", "vIdE- 42", " empty 9 "] {
            assert!(is_routing_marker(name), "{name}");
        }
        for name in [
            "",
            "vide_",
            "Empty basket",
            "Vider oven",
            "vide1x",
            "Vide1_2",
            "Empty2nd",
            "Already empty",
        ] {
            assert!(!is_routing_marker(name), "{name}");
        }
        let log = EventLog {
            traces: vec![vec![
                "Register".into(),
                "Vide1".into(),
                "Empty basket".into(),
                "Vider oven".into(),
                "Ship".into(),
            ]],
        };
        let raw = alpha(&log);
        assert!(raw.projected_activities.is_empty());
        assert_eq!(raw.project_trace(&log.traces[0]), log.traces[0]);
        assert_eq!(replay(&raw.net, &log.traces[0]), Ok(()));
        let projected = discover_cancellable(&log, &|| false).unwrap();
        for name in ["Empty basket", "Vider oven"] {
            assert!(projected.net.transitions.iter().any(|t| t.name == name && !t.silent));
        }
        assert_eq!(
            projected.project_trace(&log.traces[0]),
            ["Register", "Empty basket", "Vider oven", "Ship"]
        );
        assert_eq!(replay(&projected.net, &projected.project_trace(&log.traces[0])), Ok(()));
        assert!(replay(&projected.net, &log.traces[0]).is_err());
    }

    #[test]
    fn routing_projection_preserves_preparation_loops_source_and_net_topology() {
        let mut log = EventLog::default();
        for repeats in 1..=3 {
            for reversed in [false, true] {
                let mut trace = vec!["Take order".to_owned()];
                for index in 0..repeats {
                    trace.extend(["Prepare", "Bake", "Pack"].map(str::to_owned));
                    trace.push(if index + 1 == repeats { "Vide2" } else { "Vide1" }.into());
                }
                trace.extend(if reversed { ["Give order", "Cash"] } else { ["Cash", "Give order"] }.map(str::to_owned));
                log.traces.push(trace);
            }
        }
        let original = log.clone();
        let projected = discover_with_options_cancellable(
            &log,
            &DiscoveryOptions {
                normalize_boundaries: true,
            },
            &|| false,
        )
        .unwrap();
        assert_eq!(log, original);
        assert_eq!(projected.projected_activities, ["Vide1", "Vide2"]);
        assert!(projected.footprint.activities.contains(&"Vide1".into()));
        assert!(projected.y_w.iter().any(|pair| projected.place_name(pair).contains("Vide")));
        assert!(!projected.net.places.iter().any(|place| place.name.contains("Vide")));
        assert!(
            !projected
                .net
                .transitions
                .iter()
                .any(|transition| is_routing_marker(&transition.name))
        );
        assert_eq!(
            projected.net.transitions.iter().filter(|t| t.silent && t.name.is_empty()).count(),
            2
        );

        let boundaries = projected.boundary_activities.as_ref().unwrap();
        let augmented = EventLog {
            traces: log
                .traces
                .iter()
                .map(|trace| {
                    std::iter::once(boundaries[0].clone())
                        .chain(trace.iter().cloned())
                        .chain(std::iter::once(boundaries[1].clone()))
                        .collect()
                })
                .collect(),
        };
        let raw = alpha(&augmented);
        assert_eq!(projected.net.arcs, raw.net.arcs);
        assert_eq!(projected.net.places.len(), raw.net.places.len());
        assert_eq!(projected.net.transitions.len(), raw.net.transitions.len());
        for (projected, raw) in projected.net.transitions.iter().zip(&raw.net.transitions) {
            assert_eq!((projected.id, projected.pos), (raw.id, raw.pos));
        }
        for trace in &log.traces {
            assert_eq!(replay(&projected.net, &projected.project_trace(trace)), Ok(()));
        }
        let mut restored: AlphaResult = serde_json::from_value(serde_json::to_value(&projected).unwrap()).unwrap();
        let rebuilt = build_alpha_net(&mut restored);
        assert!(!rebuilt.transitions.iter().any(|t| is_routing_marker(&t.name)));
        assert!(!rebuilt.places.iter().any(|p| p.name.contains("Vide")));
        for trace in &log.traces {
            assert_eq!(replay(&rebuilt, &restored.project_trace(trace)), Ok(()));
        }
    }

    #[test]
    fn cached_replay_report_matches_computed_report_and_handles_missing_entries() {
        let log = EventLog::parse("ABC\nABC\nAC").unwrap();
        let result = alpha(&log);
        let replays: Vec<_> = log.variants().iter().map(|(trace, _)| replay(&result.net, trace)).collect();
        assert_eq!(result.report_with_replays(&log, &replays), result.report(&log));
        let missing = result.report_with_replays(&log, &[]);
        assert!(missing.contains(tr("mining.analysis_pending")));
        assert!(!missing.contains(" ? ok"));
        let failed = vec![Err("cached failure".to_owned()); replays.len()];
        assert!(result.report_with_replays(&log, &failed).contains("cached failure"));
    }

    #[test]
    fn replay_explores_silent_routes_and_duplicate_visible_labels() {
        let mut net = PetriNet::new("Alternative routes");
        let source = net.add_place(Pos::default(), 1);
        let first = net.add_place(Pos::default(), 0);
        let second = net.add_place(Pos::default(), 0);
        let dead = net.add_place(Pos::default(), 0);
        let ready = net.add_place(Pos::default(), 0);
        let sink = net.add_place(Pos::default(), 0);
        for place in [first, second] {
            let t = net.add_transition(Pos::default());
            net.transition_mut(t).unwrap().silent = true;
            net.add_arc(NodeRef::Place(source), NodeRef::Transition(t)).unwrap();
            net.add_arc(NodeRef::Transition(t), NodeRef::Place(place)).unwrap();
        }
        for (from, to, name) in [(first, dead, "A"), (second, ready, "A"), (ready, sink, "B")] {
            let t = net.add_transition(Pos::default());
            net.transition_mut(t).unwrap().name = name.into();
            net.add_arc(NodeRef::Place(from), NodeRef::Transition(t)).unwrap();
            net.add_arc(NodeRef::Transition(t), NodeRef::Place(to)).unwrap();
        }
        assert_eq!(replay(&net, &["A".into(), "B".into()]), Ok(()));
        assert!(replay(&net, &["B".into()]).is_err());
        assert!(replay_cancellable(&net, &["A".into()], &|| true).is_err());
    }

    #[test]
    fn replay_bounds_unbounded_silent_routing() {
        let mut net = PetriNet::new("Unbounded silent routing");
        let source = net.add_place(Pos::default(), 1);
        let growing = net.add_place(Pos::default(), 0);
        let t = net.add_transition(Pos::default());
        net.transition_mut(t).unwrap().silent = true;
        net.add_arc(NodeRef::Place(source), NodeRef::Transition(t)).unwrap();
        net.add_arc(NodeRef::Transition(t), NodeRef::Place(growing)).unwrap();
        // A returned token creates an unbounded closure once the routing has started.
        net.add_arc(NodeRef::Transition(t), NodeRef::Place(source)).unwrap();
        // Keep the input place a source for the replay convention.
        let initial = net.add_place(Pos::default(), 1);
        let enter = net.add_transition(Pos::default());
        net.transition_mut(enter).unwrap().silent = true;
        net.add_arc(NodeRef::Place(initial), NodeRef::Transition(enter)).unwrap();
        net.add_arc(NodeRef::Transition(enter), NodeRef::Place(source)).unwrap();
        assert_eq!(replay(&net, &[]), Err(tr("mining.replay_limit").into()));
    }

    #[test]
    fn csv_preserves_separators_quotes_newlines_and_case_order() {
        let log =
            EventLog::parse("\u{feff}case,activity\r\n1,\"Review, approve\"\r\n2,\"Say \"\"yes\"\"\"\r\n1,\"Next\nstep\"\r\n").unwrap();
        assert_eq!(log.traces, vec![vec!["Review, approve", "Next\nstep"], vec!["Say \"yes\""]]);
        let log = EventLog::parse("# comment\ncase;activity\n1;\"Review; approve\"\n").unwrap();
        assert_eq!(log.traces, vec![vec!["Review; approve"]]);
        let log = EventLog::parse("case\tactivity\n1\tReview\n").unwrap();
        assert_eq!(log.traces, vec![vec!["Review"]]);
    }

    #[test]
    fn csv_rejects_incomplete_records_and_invalid_quoting() {
        for source in [
            "case,activity\n1,Review\n2\n",
            "case,activity\n1,Review,extra\n",
            "case,activity\n1,\n",
            "case,activity\n,Review\n",
            "case,activity\n1,\"unfinished",
            "case,activity\n1,\"Review\"trailing",
        ] {
            assert!(EventLog::parse(source).is_err(), "{source}");
        }
    }

    fn names(r: &AlphaResult, v: &[Pair]) -> Vec<String> {
        v.iter().map(|p| r.pair(p)).collect()
    }

    /// Course example 1: ABCD, ACBD, EF (given as interleaved "case : task" lines).
    #[test]
    fn course_example_1() {
        let src = "case 1 : task A\ncase 2 : task A\ncase 3 : task A\ncase 3 : task B\ncase 1 : task B\ncase 1 : task C\n\
                   case 2 : task C\ncase 4 : task A\ncase 2 : task B\ncase 2 : task D\ncase 5 : task E\ncase 4 : task C\n\
                   case 1 : task D\ncase 3 : task C\ncase 3 : task D\ncase 4 : task B\ncase 5 : task F\ncase 4 : task D";
        let log = EventLog::parse(src).unwrap();
        let v: Vec<String> = log.variants().iter().map(|(t, _)| t.concat()).collect();
        assert_eq!(v, ["ABCD", "ACBD", "EF"]);
        let r = alpha(&log);
        let f = &r.footprint;
        assert_eq!(f.cell(0, 1), "→");
        assert_eq!(f.cell(1, 2), "||");
        assert_eq!(f.cell(3, 1), "←");
        assert_eq!(f.cell(0, 3), "#");
        assert_eq!(f.cell(0, 0), "");
        assert_eq!(r.set(&r.t_i), "{A,E}");
        assert_eq!(r.set(&r.t_o), "{D,F}");
        assert_eq!(names(&r, &r.x_w), ["({A},{B})", "({A},{C})", "({B},{D})", "({C},{D})", "({E},{F})"]);
        assert_eq!(r.x_w, r.y_w);
        assert_eq!(r.net.places.len(), 7);
        for (t, _) in log.variants() {
            assert_eq!(replay(&r.net, &t), Ok(()));
        }
    }

    /// Course example 2: ABCD, ACBD, AED.
    #[test]
    fn course_example_2() {
        let log = EventLog::parse("ABCD, ACBD, AED").unwrap();
        let r = alpha(&log);
        assert_eq!(r.set(&r.t_i), "{A}");
        assert_eq!(r.set(&r.t_o), "{D}");
        assert_eq!(r.x_w.len(), 10);
        assert_eq!(names(&r, &r.y_w), ["({A},{B,E})", "({A},{C,E})", "({B,E},{D})", "({C,E},{D})"]);
        let places: Vec<String> = r.y_w.iter().map(|p| r.place_name(p)).collect();
        assert_eq!(places, ["P(A,BE)", "P(A,CE)", "P(BE,D)", "P(CE,D)"]);
        for (t, _) in log.variants() {
            assert_eq!(replay(&r.net, &t), Ok(()));
        }
        assert!(replay(&r.net, &["A".into(), "B".into(), "D".into()]).is_err());
        assert!(crate::analysis::StructuralReport::compute(&r.net).workflow_net);
    }

    #[test]
    fn log_formats() {
        let a = EventLog::parse("Case 1, Task A    Case 3, Task A\nCase 1, Task B    Case 3, Task C\n").unwrap();
        assert_eq!(a.traces, vec![vec!["A", "B"], vec!["A", "C"]]);
        let b = EventLog::parse("register -> check -> pay\nregister -> pay").unwrap();
        assert_eq!(b.traces[0], vec!["register", "check", "pay"]);
        let c = EventLog::parse("case_id,activity,time\n1,a,0\n2,a,1\n1,b,2\n").unwrap();
        assert_eq!(c.traces, vec![vec!["a", "b"], vec!["a"]]);
        let d = EventLog::parse("XLSVKRJM   XLSKVRJM\nXWTM").unwrap();
        assert_eq!(d.traces.len(), 3);
        let xes = r#"<?xml version="1.0"?><log xes.version="1.0"><trace><string key="concept:name" value="1"/>
            <event><string key="concept:name" value="A"/></event><event><string key="concept:name" value="B"/></event></trace></log>"#;
        assert_eq!(EventLog::parse(xes).unwrap().traces, vec![vec!["A", "B"]]);
    }

    #[test]
    fn natural_order() {
        let mut v = vec!["Task 10", "Task 2", "Task 1"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["Task 1", "Task 2", "Task 10"]);
    }
}
