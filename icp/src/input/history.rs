//! Stores command history.

use std::collections::VecDeque;

pub struct History {
    entries: VecDeque<String>,
    current: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            current: 0,
        }
    }

    pub fn prev(&mut self, current: String) -> String {
        if self.entries.is_empty() {
            return current;
        }

        if self.current == 0 {
            // Store current input as a new entry, so that the user can get back
            // to editing it.
            self.entries.push_front(current);
            self.current = 1;
        } else if self.current < self.entries.len() - 1 {
            self.current += 1;
        }

        self.entries
            .get(self.current)
            .expect("`self.current` always points to a valid entry")
            .clone()
    }

    pub fn next(&mut self, current: String) -> String {
        if self.current == 0 || self.entries.is_empty() {
            return current;
        }

        if self.current == 1 {
            // We got back to the beginning of the history, so we just return
            // the "fake" entry used to preserve user input before the browse
            // operation started.
            self.current = 0;
            self.entries
                .pop_front()
                .expect("We just checked that the `entries` is non-empty")
        } else {
            self.current -= 1;
            self.entries
                .get(self.current)
                .expect("`self.current` always points to a valid entry")
                .clone()
        }
    }

    pub fn append(&mut self, input: String) {
        if self.current != 0 {
            // We were in the process of browsing the history.  Our 0th entry is
            // actually a user input we preserved.
            let _ = self.entries.pop_front();
            self.current = 0;
        }

        self.entries.push_front(input);
    }
}
