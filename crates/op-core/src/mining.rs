//! Process discovery: event logs, footprint (relation) matrix and the Alpha algorithm
//! (van der Aalst, Weijters & Maruster, 2004), with the notation used in class:
//! `>` direct succession, `→` causality, `||` parallelism, `#` independence,
//! and the 8 steps TW, TI, TO, XW, YW, PW, FW, α(W).

use crate::geom::Pos;
use crate::i18n::{tr, trf};
use crate::petri::{NodeRef, PetriNet};
use crate::xml;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

// -------------------------------------------------------------------------
// Event log

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventLog {
    /// One trace (case) = ordered list of activity names.
    pub traces: Vec<Vec<String>>,
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
            return Err(tr("The log is empty").into());
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

        // CSV with header
        if let Some(log) = Self::parse_csv(&lines) {
            return Ok(log);
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

    fn parse_csv(lines: &[&str]) -> Option<EventLog> {
        let sep = if lines[0].contains(';') { ';' } else { ',' };
        let header: Vec<String> = lines[0].split(sep).map(|s| s.trim().trim_matches('"').to_lowercase()).collect();
        let case_col = header
            .iter()
            .position(|h| h.contains("case") || h == "id" || h.contains("instance"))?;
        let act_col = header
            .iter()
            .position(|h| h.contains("activit") || h.contains("task") || h.contains("event") || h.contains("concept:name"))?;
        let mut order = Vec::new();
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for l in &lines[1..] {
            let cols: Vec<&str> = l.split(sep).map(|s| s.trim().trim_matches('"')).collect();
            let (Some(c), Some(a)) = (cols.get(case_col), cols.get(act_col)) else {
                continue;
            };
            if !map.contains_key(*c) {
                order.push(c.to_string());
            }
            map.entry(c.to_string()).or_default().push(a.to_string());
        }
        Some(EventLog {
            traces: order.into_iter().map(|c| map.remove(&c).unwrap()).collect(),
        })
    }

    /// IEEE XES (as produced by ProM): `<log><trace><event><string key="concept:name" value="A"/>`.
    pub fn from_xes(src: &str) -> Result<EventLog, String> {
        let root = xml::parse(src)?;
        if root.name != "log" {
            return Err(trf("Not an XES log (root element is <{}>)", &[&root.name]));
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug)]
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

    /// Pairs (a, b) in the given relation, a < b for symmetric relations.
    pub fn pairs(&self, r: Relation) -> Vec<(usize, usize)> {
        let n = self.len();
        let mut out = Vec::new();
        for a in 0..n {
            for b in 0..n {
                if self.rel[a][b] != r {
                    continue;
                }
                let sym = matches!(r, Relation::Parallel | Relation::Independent);
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

#[derive(Clone, Debug)]
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
        let f = &self.footprint;
        let n = |a: usize| self.name(a).to_string();
        let mut s = String::new();
        s += tr("Traces:\n");
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
        s += &trf("\nDirect successions (>): {}\n", &[&(list(f.successions(), ">"))]);
        s += &trf("Causality (→): {}\n", &[&(list(f.pairs(Relation::Causal), "→"))]);
        let par: Vec<(usize, usize)> = f
            .pairs(Relation::Parallel)
            .into_iter()
            .flat_map(|(a, b)| if a == b { vec![(a, b)] } else { vec![(a, b), (b, a)] })
            .collect();
        s += &trf("Parallelism (||): {}\n", &[&(list(par, "||"))]);
        s += &trf("Independence (#): {}\n", &[&(list(f.pairs(Relation::Independent), "#"))]);
        s += &trf("\nRelation matrix:\n{}\n", &[&f.to_text()]);
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
        s += tr("Word check:\n");
        for (t, _) in log.variants() {
            s += &format!(
                "  {} ? {}\n",
                log.format_trace(&t),
                match replay(&self.net, &t) {
                    Ok(()) => "ok".to_string(),
                    Err(e) => trf("NOT OK ({})", &[&e]),
                }
            );
        }
        s
    }
}

pub fn alpha(log: &EventLog) -> AlphaResult {
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
        if seen.len() >= MAX_X {
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

    // Step 5: maximal pairs.
    let subset = |s: &[usize], t: &[usize]| s.iter().all(|x| t.contains(x));
    let y_w: Vec<Pair> = x_w
        .iter()
        .filter(|p| !x_w.iter().any(|q| q != *p && subset(&p.0, &q.0) && subset(&p.1, &q.1)))
        .cloned()
        .collect();

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
    };
    build_net(&mut res);
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
    let mut net = PetriNet::new(tr("Discovered process (Alpha)"));

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
        net.transition_mut(id).unwrap().name = r.footprint.activities[a].clone();
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
        net.place_mut(pid).unwrap().name = r.place_name(&p);
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
pub fn replay(net: &PetriNet, trace: &[String]) -> Result<(), String> {
    let c = net.compile();
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
    for (k, act) in trace.iter().enumerate() {
        let cands: Vec<usize> = (0..net.transitions.len()).filter(|&t| &net.transitions[t].name == act).collect();
        if cands.is_empty() {
            return Err(trf("{} is not in the net", &[&act]));
        }
        let Some(&t) = cands.iter().find(|&&t| c.is_enabled(t, &m)) else {
            return Err(trf("{} not enabled at position {}", &[&act, &(k + 1)]));
        };
        m = c.fire(t, &m).unwrap();
    }
    let ok_end = sinks.iter().all(|&s| m[s] <= 1) && sinks.iter().map(|&s| m[s]).sum::<u32>() == 1;
    let leftovers: Vec<&str> = (0..m.len())
        .filter(|&p| m[p] > 0 && !sinks.contains(&p))
        .map(|p| net.places[p].name.as_str())
        .collect();
    if !ok_end {
        let k: u32 = sinks.iter().map(|&s| m[s]).sum();
        let names: Vec<&str> = sinks.iter().map(|&s| net.places[s].name.as_str()).collect();
        return Err(trf("ends with {} token(s) in {} instead of exactly 1", &[&k, &(names.join(", "))]));
    }
    if !leftovers.is_empty() {
        return Err(trf("tokens left in {}", &[&(leftovers.join(", "))]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
