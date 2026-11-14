//! Typed snapshot history with gesture transactions and an estimated memory budget.
use serde::Serialize;
use std::collections::VecDeque;

pub struct History<T: Clone + PartialEq + Serialize> {
    committed: T,
    committed_bytes: usize,
    undo: VecDeque<(T, usize)>,
    redo: VecDeque<(T, usize)>,
    transaction: Option<T>,
    bytes: usize,
    max_bytes: usize,
    generation: u64,
}

const MAX_STEPS: usize = 200;
const MAX_BYTES: usize = 64 * 1024 * 1024;

// Count serialized content without allocating a second JSON document. The factor
// accounts conservatively for container overhead; this is not a process RSS cap.
fn estimated_bytes(value: &impl Serialize) -> usize {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    if serde_json::to_writer(&mut counter, value).is_err() {
        return usize::MAX;
    }
    counter.0.saturating_mul(2).saturating_add(std::mem::size_of_val(value))
}

impl<T: Clone + PartialEq + Serialize> History<T> {
    pub fn new(initial: &T) -> Self {
        Self {
            committed: initial.clone(),
            committed_bytes: estimated_bytes(initial),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            transaction: None,
            bytes: 0,
            max_bytes: MAX_BYTES,
            generation: 0,
        }
    }

    /// Changes during gestures also advance the token; viewport-only frames do not.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn trim(&mut self) {
        while self.bytes > self.max_bytes || self.undo.len() + self.redo.len() > MAX_STEPS {
            let removed = self.undo.pop_front().or_else(|| self.redo.pop_front());
            let Some((_, bytes)) = removed else { break };
            self.bytes -= bytes;
        }
    }

    pub fn sync(&mut self, current: &T, interacting: bool) {
        let changed = *current != self.committed;
        if changed {
            self.generation = self.generation.wrapping_add(1);
        }
        if interacting {
            self.begin(current);
            return;
        }
        self.transaction = None;
        if !changed {
            return;
        }
        let previous = std::mem::replace(&mut self.committed, current.clone());
        let previous_bytes = std::mem::replace(&mut self.committed_bytes, estimated_bytes(current));
        self.bytes -= self.redo.iter().map(|(_, bytes)| bytes).sum::<usize>();
        self.redo.clear();
        self.undo.push_back((previous, previous_bytes));
        self.bytes = self.bytes.saturating_add(previous_bytes);
        self.trim();
    }

    pub fn begin(&mut self, _current: &T) {
        if self.transaction.is_none() {
            self.transaction = Some(self.committed.clone());
        }
    }

    pub fn cancel(&mut self, current: &mut T) {
        if let Some(initial) = self.transaction.take() {
            *current = initial;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: &mut T) {
        self.sync(current, false);
        if let Some((previous, bytes)) = self.undo.pop_back() {
            self.bytes -= bytes;
            self.bytes = self.bytes.saturating_add(self.committed_bytes);
            self.redo
                .push_back((std::mem::replace(current, previous.clone()), self.committed_bytes));
            self.committed = previous;
            self.committed_bytes = bytes;
            self.generation = self.generation.wrapping_add(1);
            self.trim();
        }
    }

    pub fn redo(&mut self, current: &mut T) {
        if let Some((next, bytes)) = self.redo.pop_back() {
            self.bytes -= bytes;
            self.bytes = self.bytes.saturating_add(self.committed_bytes);
            self.undo
                .push_back((std::mem::replace(current, next.clone()), self.committed_bytes));
            self.committed = next;
            self.committed_bytes = bytes;
            self.generation = self.generation.wrapping_add(1);
            self.trim();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_budget_bounds_undo_and_redo_without_serializing_idle_frames() {
        let mut value = "x".repeat(100);
        let mut history = History::new(&value);
        history.max_bytes = 1000;
        let generation = history.generation();
        history.sync(&value, false);
        assert_eq!(history.generation(), generation);
        for index in 0..100 {
            value = format!("{index:03}{}", "x".repeat(100));
            history.sync(&value, false);
            assert!(history.bytes <= history.max_bytes);
        }
        assert!(history.undo.len() < 100);
        let mut undo_count = 0;
        while history.can_undo() {
            history.undo(&mut value);
            undo_count += 1;
            assert!(history.bytes <= history.max_bytes);
        }
        assert!(undo_count > 0);
        while history.can_redo() {
            history.redo(&mut value);
            assert!(history.bytes <= history.max_bytes);
        }
        assert!(value.starts_with("099"));
    }

    #[test]
    fn undo_redo() {
        let mut v = 1;
        let mut h = History::new(&v);
        v = 2;
        h.sync(&v, false);
        v = 3;
        h.sync(&v, true); // interacting: not committed yet
        h.sync(&v, false);
        h.undo(&mut v);
        assert_eq!(v, 2);
        h.undo(&mut v);
        assert_eq!(v, 1);
        h.redo(&mut v);
        assert_eq!(v, 2);
    }

    #[test]
    fn gesture_commit_and_cancel_preserve_history() {
        let mut value = vec![1];
        let mut h = History::new(&value);
        for n in 2..20 {
            value[0] = n;
            h.sync(&value, true);
        }
        h.sync(&value, false);
        h.undo(&mut value);
        assert_eq!(value, [1]);
        assert!(!h.can_undo());
        assert!(h.can_redo());
        value[0] = 42;
        h.sync(&value, true);
        h.cancel(&mut value);
        assert_eq!(value, [1]);
        assert!(h.can_redo());
        h.redo(&mut value);
        assert_eq!(value, [19]);
    }
}
