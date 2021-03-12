pub mod commands;
pub mod input;

pub use input::{Input, Prompt};

use std::cell::RefCell;
use std::iter::Extend;
use std::rc::Rc;

pub trait TerminalContentRef: Clone {
    fn push(&mut self, line: String);

    fn extend<Lines>(&mut self, lines: Lines)
    where
        Lines: IntoIterator<Item = String>;
}

impl TerminalContentRef for Rc<RefCell<Vec<String>>> {
    fn push(&mut self, line: String) {
        self.borrow_mut().push(line)
    }

    fn extend<Lines>(&mut self, lines: Lines)
    where
        Lines: IntoIterator<Item = String>,
    {
        self.borrow_mut().extend(lines);
    }
}

pub fn str_byte_pos(s: &str, pos: usize) -> usize {
    s.char_indices().nth(pos).map_or(s.len(), |(i, _)| i)
}
