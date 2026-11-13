//! Snapshot-based undo/redo.
//!
//! The editor mutates its model freely; once per frame [`History::sync`] is
//! called and, when no interaction is in progress (no drag, no text edit),
//! any difference with the last committed state becomes one undo step.

pub struct History<T: Clone + PartialEq> {
    committed: T,
    undo: Vec<T>,
    redo: Vec<T>,
}

const MAX_STEPS: usize = 200;

impl<T: Clone + PartialEq> History<T> {
    pub fn new(initial: &T) -> Self {
        Self {
            committed: initial.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn sync(&mut self, current: &T, interacting: bool) {
        if interacting || *current == self.committed {
            return;
        }
        let prev = std::mem::replace(&mut self.committed, current.clone());
        self.undo.push(prev);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
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
}
