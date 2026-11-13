//! Snapshot-based undo/redo.
//!
//! The editor mutates its model freely; once per frame [`History::sync`] is
//! called and, when no interaction is in progress (no drag, no text edit),
//! any difference with the last committed state becomes one undo step.

pub struct History<T: Clone + PartialEq> {
    committed: T,
    undo: Vec<T>,
    redo: Vec<T>,
    transaction: Option<T>,
}

const MAX_STEPS: usize = 200;

impl<T: Clone + PartialEq> History<T> {
    pub fn new(initial: &T) -> Self {
        Self {
            committed: initial.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
            transaction: None,
        }
    }

    pub fn sync(&mut self, current: &T, interacting: bool) {
        if interacting {
            self.begin(current);
            return;
        }
        self.transaction = None;
        if *current == self.committed {
            return;
        }
        let prev = std::mem::replace(&mut self.committed, current.clone());
        self.undo.push(prev);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// The committed state precedes any in-flight pointer/text mutations.
    pub fn begin(&mut self, _current: &T) {
        if self.transaction.is_none() {
            self.transaction = Some(self.committed.clone());
        }
    }

    /// Roll back the gesture without adding an undo step or clearing redo.
    pub fn cancel(&mut self, current: &mut T) {
        if let Some(initial) = self.transaction.take() {
            *current = initial;
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: &mut T) {
        // Commit pending changes first so they can be redone.
        self.sync(current, false);
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(current, prev.clone()));
            self.committed = prev;
        }
    }

    pub fn redo(&mut self, current: &mut T) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(current, next.clone()));
            self.committed = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
