// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
