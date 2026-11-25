//! Random Alpha exercises.
//!
//! An exercise log comes from a random block-structured process: sequences,
//! exclusive choices, parallel branches and, at the hard level, loops whose
//! cycles contain at least three activities. Activities are unique one-letter
//! names, and every case starts with `A` and ends with the last letter, so the
//! initial and final places of α(W) are correct by construction.
//!
//! The traces cover every direct succession of the process, which makes the log
//! complete for Alpha. A candidate is kept only when α(W) is a sound workflow net
//! whose language, up to the longest trace that repeats each loop at most once,
//! equals the language of the process. Otherwise another process is drawn.
//! Generation is deterministic for a given seed and difficulty.

use super::{AlphaResult, EventLog, Pair, alpha};
use crate::analysis::{Soundness, soundness};
use crate::i18n::key;
use crate::petri::{CompiledNet, Marking, NodeRef, PetriNet};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Size and constructs of a generated exercise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    /// 4–6 activities; one choice or parallel block of single activities.
    #[default]
    Easy,
    /// 6–8 activities; nested choices and parallel branches.
    Medium,
    /// 7–10 activities; nested blocks including a loop.
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    /// Stable message key of the level's name.
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => key("mining.exercise.easy"),
            Difficulty::Medium => key("mining.exercise.medium"),
            Difficulty::Hard => key("mining.exercise.hard"),
        }
    }

    fn profile(self) -> Profile {
        match self {
            Difficulty::Easy => Profile {
                activities: (4, 6),
                segment: 3,
                depth: 1,
                loops: false,
                max_traces: 4,
            },
            Difficulty::Medium => Profile {
                activities: (6, 8),
                segment: 4,
                depth: 2,
                loops: false,
                max_traces: 6,
            },
            Difficulty::Hard => Profile {
                activities: (7, 10),
                segment: 6,
                depth: 3,
                loops: true,
                max_traces: 8,
            },
        }
    }

    /// Constructs that make the level worth practising.
    fn accepts(self, process: &Block) -> bool {
        let choice = process.contains(Operator::Choice);
        let parallel = process.contains(Operator::Parallel);
        match self {
            Difficulty::Easy => choice || parallel,
            Difficulty::Medium => choice && parallel,
            Difficulty::Hard => process.contains(Operator::Loop) && (choice || parallel),
        }
    }
}

/// A generated log, ready to be studied with Alpha.
#[derive(Clone, Debug, PartialEq)]
pub struct Exercise {
    pub difficulty: Difficulty,
    pub seed: u64,
    pub log: EventLog,
}

impl Exercise {
    /// One compact word per trace and line, as in `ABCD`.
    pub fn source(&self) -> String {
        self.log.traces.iter().map(|trace| trace.concat() + "\n").collect()
    }
}

/// Draws processes from `seed` until one yields a log that Alpha rediscovers.
pub fn generate(difficulty: Difficulty, seed: u64) -> Exercise {
    let mut generator = Generator::new(difficulty, seed);
    let log = (0..ATTEMPTS)
        .find_map(|_| generator.attempt())
        .or_else(|| generator.realize(&Block::fallback()))
        .expect("the fallback process is a valid Alpha exercise");
    Exercise { difficulty, seed, log }
}

/// Step 5 explanation: pairs of XW that are not maximal, each with a pair of YW
/// that contains it. At most `limit` pairs are returned, in XW order; the total
/// number of removed pairs is `x_w.len() - y_w.len()`.
pub fn removed_pairs(r: &AlphaResult, limit: usize) -> Vec<(&Pair, &Pair)> {
    let maximal: HashSet<&Pair> = r.y_w.iter().collect();
    r.x_w
        .iter()
        .filter(|pair| !maximal.contains(pair))
        .filter_map(|pair| r.y_w.iter().find(|wider| includes(wider, pair)).map(|wider| (pair, wider)))
        .take(limit)
        .collect()
}

fn includes(wider: &Pair, pair: &Pair) -> bool {
    pair.0.iter().all(|a| wider.0.contains(a)) && pair.1.iter().all(|b| wider.1.contains(b))
}

const ATTEMPTS: usize = 400;
/// Bounded languages larger than this are rejected rather than enumerated.
const LANGUAGE_LIMIT: usize = 2_000;
/// Memoized (marking, remaining length) states when comparing α(W) with the process.
const NET_STATE_LIMIT: usize = 20_000;
const SOUNDNESS_STATES: usize = 5_000;

struct Profile {
    /// Number of activities, including the first and the last one.
    activities: (usize, usize),
    /// Largest block of the main sequence.
    segment: usize,
    /// Nesting depth of blocks inside the main sequence.
    depth: usize,
    loops: bool,
    max_traces: usize,
}

/// SplitMix64: small, portable and reproducible on native and WebAssembly targets.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`, for `n > 0`.
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// Uniform in `low..=high`.
    fn range(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high - low + 1)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    Sequence,
    Choice,
    Parallel,
    Loop,
}

/// Process tree with unique activities, numbered in reading order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Block {
    Activity(usize),
    Sequence(Vec<Block>),
    Choice(Vec<Block>),
    Parallel(Vec<Block>),
    /// `body (redo body)*`.
    Loop(Box<Block>, Box<Block>),
}

type Trace = Vec<usize>;

impl Block {
    /// `A ; ((B || C) × D) ; E`: the classic course log `ABCE, ACBE, ADE`.
    fn fallback() -> Block {
        use Block::*;
        let mut process = Sequence(vec![
            Activity(0),
            Choice(vec![Parallel(vec![Activity(0), Activity(0)]), Activity(0)]),
            Activity(0),
        ]);
        process.number(&mut 0);
        process
    }

    fn children(&self) -> Vec<&Block> {
        match self {
            Block::Activity(_) => Vec::new(),
            Block::Sequence(c) | Block::Choice(c) | Block::Parallel(c) => c.iter().collect(),
            Block::Loop(body, redo) => vec![body, redo],
        }
    }

    fn number(&mut self, next: &mut usize) {
        match self {
            Block::Activity(a) => {
                *a = *next;
                *next += 1;
            }
            Block::Sequence(c) | Block::Choice(c) | Block::Parallel(c) => c.iter_mut().for_each(|b| b.number(next)),
            Block::Loop(body, redo) => {
                body.number(next);
                redo.number(next);
            }
        }
    }

    fn activities(&self) -> usize {
        match self {
            Block::Activity(_) => 1,
            _ => self.children().into_iter().map(Block::activities).sum(),
        }
    }

    fn contains(&self, operator: Operator) -> bool {
        let own = match self {
            Block::Activity(_) => None,
            Block::Sequence(_) => Some(Operator::Sequence),
            Block::Choice(_) => Some(Operator::Choice),
            Block::Parallel(_) => Some(Operator::Parallel),
            Block::Loop(..) => Some(Operator::Loop),
        };
        own == Some(operator) || self.children().into_iter().any(|c| c.contains(operator))
    }

    /// Length of the shortest trace.
    fn min_len(&self) -> usize {
        match self {
            Block::Activity(_) => 1,
            Block::Sequence(c) | Block::Parallel(c) => c.iter().map(Block::min_len).sum(),
            Block::Choice(c) => c.iter().map(Block::min_len).min().unwrap_or(0),
            Block::Loop(body, _) => body.min_len(),
        }
    }

    /// Length of the longest trace when every loop repeats at most once.
    fn pass_len(&self) -> usize {
        match self {
            Block::Activity(_) => 1,
            Block::Sequence(c) | Block::Parallel(c) => c.iter().map(Block::pass_len).sum(),
            Block::Choice(c) => c.iter().map(Block::pass_len).max().unwrap_or(0),
            Block::Loop(body, redo) => 2 * body.pass_len() + redo.pass_len(),
        }
    }

    /// Every trace of at most `bound` activities, or `None` beyond `limit` traces.
    fn language(&self, bound: usize, limit: usize) -> Option<BTreeSet<Trace>> {
        let out = match self {
            Block::Activity(a) => (bound >= 1).then(|| vec![*a]).into_iter().collect(),
            Block::Choice(children) => {
                let mut out = BTreeSet::new();
                for child in children {
                    out.extend(child.language(bound, limit)?);
                    if out.len() > limit {
                        return None;
                    }
                }
                out
            }
            Block::Sequence(children) | Block::Parallel(children) => {
                let parallel = matches!(self, Block::Parallel(_));
                let mut rest: usize = children.iter().map(Block::min_len).sum();
                let mut out = BTreeSet::from([Trace::new()]);
                for child in children {
                    rest -= child.min_len();
                    let tails = child.language(bound.saturating_sub(rest), limit)?;
                    let mut next = BTreeSet::new();
                    for head in &out {
                        for tail in tails.iter().filter(|tail| head.len() + tail.len() + rest <= bound) {
                            if parallel {
                                interleave(head, tail, &mut Vec::new(), &mut next);
                            } else {
                                next.insert([head.as_slice(), tail].concat());
                            }
                            if next.len() > limit {
                                return None;
                            }
                        }
                    }
                    out = next;
                }
                out
            }
            Block::Loop(body, redo) => {
                let bodies = body.language(bound, limit)?;
                let redos = redo.language(bound, limit)?;
                let mut out = bodies.clone();
                let mut frontier: Vec<Trace> = bodies.iter().cloned().collect();
                while !frontier.is_empty() {
                    let mut next = Vec::new();
                    for start in &frontier {
                        for again in &redos {
                            for body in bodies.iter().filter(|b| start.len() + again.len() + b.len() <= bound) {
                                let trace = [start.as_slice(), again, body].concat();
                                if out.insert(trace.clone()) {
                                    next.push(trace);
                                }
                                if out.len() > limit {
                                    return None;
                                }
                            }
                        }
                    }
                    frontier = next;
                }
                out
            }
        };
        Some(out)
    }
}

/// Adds every interleaving of `a` and `b` that preserves both orders.
fn interleave(a: &[usize], b: &[usize], prefix: &mut Vec<usize>, out: &mut BTreeSet<Trace>) {
    match (a.split_first(), b.split_first()) {
        (None, _) => {
            out.insert([prefix.as_slice(), b].concat());
        }
        (_, None) => {
            out.insert([prefix.as_slice(), a].concat());
        }
        (Some((&x, rest_a)), Some((&y, rest_b))) => {
            prefix.push(x);
            interleave(rest_a, b, prefix, out);
            prefix.pop();
            prefix.push(y);
            interleave(a, rest_b, prefix, out);
            prefix.pop();
        }
    }
}

struct Generator {
    rng: Rng,
    difficulty: Difficulty,
    profile: Profile,
}

impl Generator {
    fn new(difficulty: Difficulty, seed: u64) -> Self {
        Self {
            rng: Rng(seed),
            difficulty,
            profile: difficulty.profile(),
        }
    }

    fn attempt(&mut self) -> Option<EventLog> {
        let process = self.process();
        if !self.difficulty.accepts(&process) {
            return None;
        }
        self.realize(&process)
    }

    /// `first ; blocks… ; last`, numbered in reading order.
    fn process(&mut self) -> Block {
        let (low, high) = self.profile.activities;
        let mut remaining = self.rng.range(low, high) - 2;
        let mut parts = vec![Block::Activity(0)];
        while remaining > 0 {
            let size = self.rng.range(1, remaining.min(self.profile.segment));
            parts.push(self.block(size, self.profile.depth, Operator::Sequence));
            remaining -= size;
        }
        parts.push(Block::Activity(0));
        let mut process = Block::Sequence(parts);
        process.number(&mut 0);
        process
    }

    fn block(&mut self, size: usize, depth: usize, parent: Operator) -> Block {
        if size == 1 {
            return Block::Activity(0);
        }
        let operators: Vec<Operator> = [Operator::Choice, Operator::Parallel, Operator::Sequence, Operator::Loop]
            .into_iter()
            .filter(|&operator| {
                operator != parent
                    && match operator {
                        // A sequence of single activities belongs to the enclosing sequence.
                        Operator::Sequence => depth > 1,
                        Operator::Loop => self.profile.loops && depth > 1 && size >= 3,
                        Operator::Choice | Operator::Parallel => true,
                    }
            })
            .collect();
        let operator = operators[self.rng.below(operators.len())];
        let sizes = if depth <= 1 {
            vec![1; size]
        } else if operator == Operator::Loop {
            let body = self.rng.range(1, size - 1);
            vec![body, size - body]
        } else {
            let parts = self.rng.range(2, size.min(3));
            self.split(size, parts)
        };
        let mut children: Vec<Block> = sizes.into_iter().map(|n| self.block(n, depth - 1, operator)).collect();
        match operator {
            Operator::Sequence => Block::Sequence(children),
            Operator::Choice => Block::Choice(children),
            Operator::Parallel => Block::Parallel(children),
            Operator::Loop => {
                let redo = children.pop().expect("loop redo");
                let body = children.pop().expect("loop body");
                // Alpha cannot distinguish cycles of one or two activities from parallelism.
                if body.min_len() + redo.min_len() < 3 {
                    Block::Sequence(vec![body, redo])
                } else {
                    Block::Loop(Box::new(body), Box::new(redo))
                }
            }
        }
    }

    /// Random composition of `size` into `parts` positive sizes.
    fn split(&mut self, size: usize, parts: usize) -> Vec<usize> {
        let mut sizes = vec![1; parts];
        for _ in parts..size {
            let index = self.rng.below(parts);
            sizes[index] += 1;
        }
        sizes
    }

    /// Chooses traces covering every direct succession, then checks α(W).
    fn realize(&mut self, process: &Block) -> Option<EventLog> {
        let bound = process.pass_len();
        let language = process.language(bound, LANGUAGE_LIMIT)?;
        let mut traces = self.cover(&language);
        if traces.len() < 2 || traces.len() > self.profile.max_traces {
            return None;
        }
        traces.sort();
        let names: Vec<String> = (0..process.activities()).map(|a| char::from(b'A' + a as u8).to_string()).collect();
        let named = |trace: &Trace| -> Vec<String> { trace.iter().map(|&a| names[a].clone()).collect() };
        let log = EventLog {
            traces: traces.iter().map(named).collect(),
        };
        let discovered = alpha(&log);
        if discovered.x_truncated {
            return None;
        }
        let expected: BTreeSet<Vec<String>> = language.iter().map(named).collect();
        if !net_language_is(&discovered.net, &expected, bound) {
            return None;
        }
        matches!(soundness(&discovered.net, SOUNDNESS_STATES), Soundness::Sound).then_some(log)
    }

    /// Greedy cover of the direct successions; ties prefer shorter traces.
    fn cover(&mut self, language: &BTreeSet<Trace>) -> Vec<Trace> {
        let successions = |trace: &Trace| -> Vec<(usize, usize)> { trace.windows(2).map(|w| (w[0], w[1])).collect() };
        let needed: HashSet<(usize, usize)> = language.iter().flat_map(successions).collect();
        let mut candidates: Vec<&Trace> = language.iter().collect();
        self.rng.shuffle(&mut candidates);
        let mut covered = HashSet::new();
        let mut chosen = Vec::new();
        while covered.len() < needed.len() {
            let best = candidates
                .iter()
                .copied()
                .max_by_key(|trace| {
                    let gain = successions(trace).into_iter().filter(|s| !covered.contains(s)).count();
                    (gain, std::cmp::Reverse(trace.len()))
                })
                .expect("the language covers its own successions");
            covered.extend(successions(best));
            chosen.push(best.clone());
        }
        // A single trace can cover a loop; a second, shorter case reads more like a log.
        if chosen.len() == 1
            && let Some(shortest) = candidates
                .iter()
                .copied()
                .filter(|trace| **trace != chosen[0])
                .min_by_key(|trace| trace.len())
        {
            chosen.push(shortest.clone());
        }
        chosen
    }
}

/// True when the complete firing sequences of at most `bound` transitions, from
/// the initial marking to one token in the only sink place, are exactly
/// `expected`. Transitions are identified by their unique activity names, so
/// accepting every expected trace and counting the same number of complete
/// sequences proves equality without enumerating the net's interleavings.
fn net_language_is(net: &PetriNet, expected: &BTreeSet<Vec<String>>, bound: usize) -> bool {
    let compiled = net.compile();
    let sinks: Vec<usize> = (0..net.places.len())
        .filter(|&p| !net.arcs.iter().any(|a| a.from == NodeRef::Place(net.places[p].id)))
        .collect();
    let [sink] = sinks.as_slice() else {
        return false;
    };
    let complete = |marking: &[u32]| marking.iter().enumerate().all(|(p, &tokens)| tokens == u32::from(p == *sink));
    let mut transitions = HashMap::new();
    for (index, transition) in net.transitions.iter().enumerate() {
        if transition.silent || transitions.insert(transition.name.as_str(), index).is_some() {
            return false;
        }
    }
    let accepted = |trace: &Vec<String>| {
        let mut marking = net.initial_marking();
        for activity in trace {
            let Some(next) = transitions.get(activity.as_str()).and_then(|&t| compiled.fire(t, &marking)) else {
                return false;
            };
            marking = next;
        }
        complete(&marking)
    };
    if !expected.iter().all(accepted) {
        return false;
    }
    let mut counts = HashMap::new();
    count_sequences(&compiled, net.initial_marking(), bound, &complete, &mut counts) == Some(expected.len() as u64)
}

/// Complete sequences of at most `steps` transitions from `marking`, memoized by
/// marking and remaining length. `None` once the explored state space is too large.
fn count_sequences(
    net: &CompiledNet,
    marking: Marking,
    steps: usize,
    complete: &impl Fn(&[u32]) -> bool,
    counts: &mut HashMap<(Marking, usize), u64>,
) -> Option<u64> {
    if complete(&marking) {
        // The sink has no output arcs and every other place is empty: nothing is enabled.
        return Some(1);
    }
    if steps == 0 {
        return Some(0);
    }
    let state = (marking, steps);
    if let Some(&known) = counts.get(&state) {
        return Some(known);
    }
    if counts.len() >= NET_STATE_LIMIT {
        return None;
    }
    let mut total = 0u64;
    for t in 0..net.transition_count() {
        if let Some(next) = net.fire(t, &state.0) {
            total = total.saturating_add(count_sequences(net, next, steps - 1, complete, counts)?);
        }
    }
    counts.insert(state, total);
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mining::{Relation, replay};

    fn rediscovered(exercise: &Exercise) -> AlphaResult {
        let parsed = EventLog::parse(&exercise.source()).unwrap();
        assert_eq!(parsed, exercise.log, "the source text reads back as the same log");
        let r = alpha(&parsed);
        assert!(!r.x_truncated);
        for trace in &parsed.traces {
            assert_eq!(replay(&r.net, trace), Ok(()), "{:?}: {trace:?}", exercise.source());
        }
        assert!(matches!(soundness(&r.net, SOUNDNESS_STATES), Soundness::Sound));
        r
    }

    #[test]
    fn every_level_generates_valid_exercises_without_the_fallback() {
        for difficulty in Difficulty::ALL {
            let profile = difficulty.profile();
            for seed in 0..120 {
                let mut generator = Generator::new(difficulty, seed);
                let log = (0..ATTEMPTS)
                    .find_map(|_| generator.attempt())
                    .unwrap_or_else(|| panic!("{difficulty:?} seed {seed} needed the fallback"));
                let exercise = generate(difficulty, seed);
                assert_eq!(exercise.log, log, "generation is deterministic");
                let r = rediscovered(&exercise);
                let activities = r.footprint.len();
                assert!(
                    (profile.activities.0..=profile.activities.1).contains(&activities),
                    "{difficulty:?} seed {seed}: {activities} activities"
                );
                assert!((2..=profile.max_traces).contains(&exercise.log.traces.len()));
                assert_eq!(r.t_i.len(), 1, "a single initial activity");
                assert_eq!(r.t_o.len(), 1, "a single final activity");
                assert!(!r.footprint.pairs(Relation::Causal).is_empty());
            }
        }
    }

    #[test]
    fn levels_practise_their_constructs() {
        let parallel = |r: &AlphaResult| !r.footprint.pairs(Relation::Parallel).is_empty();
        // A loop of length three or more appears as a causal cycle back to an earlier activity.
        let looping = |r: &AlphaResult| r.footprint.pairs(Relation::Causal).iter().any(|&(a, b)| b < a);
        let mut seen = (false, false);
        for seed in 0..40 {
            let r = rediscovered(&generate(Difficulty::Medium, seed));
            assert!(parallel(&r), "medium seed {seed} has parallel branches");
            let r = rediscovered(&generate(Difficulty::Hard, seed));
            assert!(looping(&r), "hard seed {seed} has a loop");
            let easy = rediscovered(&generate(Difficulty::Easy, seed));
            seen.0 |= parallel(&easy);
            seen.1 |= easy.y_w.iter().any(|(a, b)| a.len() > 1 || b.len() > 1);
        }
        assert!(seen.0 && seen.1, "easy exercises include parallel and choice blocks");
    }

    #[test]
    fn seeds_vary_the_exercises() {
        let distinct: HashSet<String> = (0..30).map(|seed| generate(Difficulty::Medium, seed).source()).collect();
        assert!(distinct.len() > 20, "{} distinct exercises", distinct.len());
    }

    #[test]
    fn fallback_is_the_classic_course_log() {
        let mut generator = Generator::new(Difficulty::Easy, 0);
        let log = generator.realize(&Block::fallback()).unwrap();
        assert_eq!(log, EventLog::parse("ABCE ACBE ADE").unwrap());
    }

    #[test]
    fn short_loops_and_parallel_boundaries_are_rejected() {
        use Block::*;
        let mut generator = Generator::new(Difficulty::Hard, 0);
        let mut check = |mut process: Block| {
            process.number(&mut 0);
            generator.realize(&process)
        };
        // A (B C)* D with a cycle of two activities: B||C for Alpha.
        assert!(
            check(Sequence(vec![
                Activity(0),
                Loop(Box::new(Activity(0)), Box::new(Activity(0))),
                Activity(0)
            ]))
            .is_none()
        );
        // Parallel initial activities share the single source place iw.
        assert!(check(Sequence(vec![Parallel(vec![Activity(0), Activity(0)]), Activity(0)])).is_none());
        // A cycle of three activities is rediscovered.
        let three = Loop(Box::new(Sequence(vec![Activity(0), Activity(0)])), Box::new(Activity(0)));
        let log = check(Sequence(vec![Activity(0), three, Activity(0)])).unwrap();
        assert_eq!(log, EventLog::parse("ABCDBCE ABCE").unwrap());
    }

    #[test]
    fn removed_pairs_explain_maximality() {
        let r = alpha(&EventLog::parse("ABCD ACBD AED").unwrap());
        let removed = removed_pairs(&r, usize::MAX);
        assert_eq!(removed.len(), r.x_w.len() - r.y_w.len());
        for (pair, wider) in &removed {
            assert!(!r.y_w.contains(pair) && r.y_w.contains(wider) && includes(wider, pair));
        }
        let names: Vec<String> = removed.iter().map(|(p, q)| format!("{} ⊂ {}", r.pair(p), r.pair(q))).collect();
        assert!(names.contains(&"({A},{B}) ⊂ ({A},{B,E})".to_string()), "{names:?}");
        assert_eq!(removed_pairs(&r, 2).len(), 2);
    }
}
