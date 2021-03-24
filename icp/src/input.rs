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

//! This module deals with parsing user input, providing completions and hints
//! for incomplete input.

pub mod arg_parser;
pub mod command_parser;
pub mod common_prefix;

mod history;

use std::mem::replace;

use crate::commands::table::CommandsTable;
use crate::commands::{EndOfLineHint, Executor, ParseRes};
use crate::str_byte_pos;

pub use common_prefix::common_prefix;

use history::History;

/// Prompt text may be different depending on whether the entered text forms a
/// complete command or not.  Fields specify different prompts to be shown
/// depending on the texted currently typed by the users.
#[derive(PartialEq, Clone, Debug)]
pub struct Prompt {
    /// No text has been entered so far.
    ///
    /// This prompt is shown when [`Input::input`] is empty.
    pub empty: String,

    /// Text is an incomplete command that may be extended to form a complete
    /// command.
    ///
    /// This prompt is shown when [`Input::command`] is `None`, and
    /// `Input::suggestions` is non-empty.
    pub incomplete: String,

    /// Entered command is invalid.  There are no completions that would make it
    /// valid.
    ///
    /// This prompt is shown when [`Input::command`] is `None`, and
    /// [`Input::suggestions`] is empty.
    pub invalid: String,

    /// Text is a valid command that will be executed when the user presses
    /// "Enter".
    ///
    /// This prompt is shown when [`Input::command`] is a `Just`.
    pub complete: String,
}

/// Holds an input the user has provided so far, calculates completions, and can
/// execute a command if the input forms a full command.
pub struct Input {
    /// Commands that can be executed through this input.
    commands: CommandsTable,

    /// Prompt to show before the user input.
    prompt: Prompt,

    /// All the text the user has typed so far.
    input: String,

    /// Cursor position within `input` in characters, not bytes.
    // TODO: Current implementation is incomplete.  Proper handing of Unicode
    // input is complex and should probably be done by a dedicated library, such
    // as `linefeed`.  Going between characters (it is actually better to
    // operate on grapheme clusters) and bytes is not very efficient.  But
    // cursor movement is easier when `pos` is in characters.
    //
    // Proper solution seems to be use grapheme clusters for `pos`, in which
    // case `unicode_segmentation` is a library that should be very helpful.
    pos: usize,

    /// Completion hint that is shown to the right of the cursor.  It should be
    /// indicative of what will happen when the user presses the completion key.
    /// It should start with the text that will be inserted when the completion
    /// key is pressed, but may contain extra information.
    ///
    /// `completion` is the text to be inserted when the completion key is
    /// pressed.
    ///
    /// `input` will be split in two parts when shown.  `input` content up to
    /// and including the `pos` character, then the `inline_hint` content, and
    /// then everything after the `pos` character from `input`.
    inline_hint: Option<String>,

    /// A string to be inserted when the user presses the completion key.
    /// Ideally this should be based on the `suggestions` value - either the
    /// common prefix or the first entry, depending on what makes more sense for
    /// the command at hand.  It might also be empty, even when `suggestions` is
    /// not, to indicate that there is not preferred completion.
    ///
    /// Having this value separate from the `inline_hint` allows for the hint to
    /// provide additional information compared to what will actually be
    /// inserted.  But if this value is not `None`, it is strongly recommended
    /// for  the `inline_hint` to contain this value as a prefix.
    completion: Option<String>,

    /// Single line hint to be shown at the end of the user input, separated by
    /// a white space.  Generally this should be some "quick help" about the
    /// whole command been typed.
    end_of_line_hint: Option<EndOfLineHint>,

    /// A list of all the possible ways to extend the currently input text at
    /// point indicated by `pos` to make it closer to an executable command.
    /// This may be non-empty even if `command` is a `Just`.  Suggestions are
    /// shown below the input.
    ///
    /// In case a full command has not been entered yet, or if the cursor is in
    /// the command part of the input, this list should contain all the possible
    /// commands that start with the same prefix as the part that was already
    /// typed.
    ///
    /// If a full command has been already typed, this list should list ways to
    /// complete the text of the command argument that contains the cursor.
    suggestions: Vec<String>,

    /// A free form text regarding the current command usage.  If specified it
    /// is shown below the `suggestions` (if any).  It is recommended that this
    /// text contains an "outline" of the command, if a full command has been
    /// entered and provide details on the argument that contains the input
    /// position.
    ///
    /// If a full command has not been entered yet, this text should contain the
    /// usage details of the first selected command.
    usage: Option<String>,

    /// If the currently entered text (`input`) forms a full command, this is
    /// the command that matches the entered test.  When this field is not
    /// `None` it means we can execute the entered text, and when it is `None`,
    /// it means there is nothing we can do with whatever is currently entered.
    command: Option<Box<dyn Executor>>,

    /// Commands that have been executed through this input.
    history: History,
}

impl Input {
    pub fn new(prompt: Prompt, commands: CommandsTable) -> Self {
        let usage = Some(commands.default_usage());
        Input {
            commands,
            prompt,
            input: String::new(),
            pos: 0,
            inline_hint: None,
            completion: None,
            end_of_line_hint: None,
            suggestions: vec![],
            usage,
            command: None,
            history: History::new(),
        }
    }

    pub fn execute(&mut self) {
        if let Some(command) = self.command.take() {
            self.history.append(self.input.clone());
            self.input.clear();
            self.pos = 0;
            self.update();
            (command)();
        }
    }

    pub fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn inline_hint(&self) -> Option<&str> {
        self.inline_hint.as_deref()
    }

    pub fn completion(&self) -> Option<&str> {
        self.completion.as_deref()
    }

    pub fn end_of_line_hint(&self) -> Option<&EndOfLineHint> {
        self.end_of_line_hint.as_ref()
    }

    pub fn suggestions(&self) -> &[String] {
        self.suggestions.as_slice()
    }

    pub fn usage(&self) -> Option<&str> {
        self.usage.as_deref()
    }

    pub fn command(&self) -> Option<&dyn Executor> {
        self.command.as_deref()
    }

    pub fn cursor_left(&mut self) {
        if self.pos == 0 {
            return;
        }

        self.pos -= 1;
        self.update();
    }

    pub fn cursor_right(&mut self) {
        let char_len = self.input_char_len();
        if self.pos >= char_len {
            return;
        }

        self.pos += 1;
        self.update();
    }

    #[allow(unused)]
    pub fn cursor_word_left(&mut self) {
        panic!("cursor_word_left is not implemented")
    }

    #[allow(unused)]
    pub fn cursor_word_right(&mut self) {
        panic!("cursor_word_left is not implemented")
    }

    pub fn cursor_end(&mut self) {
        let char_len = self.input_char_len();
        if self.pos >= char_len {
            return;
        }

        self.pos = char_len;
        self.update();
    }

    pub fn cursor_start(&mut self) {
        if self.pos == 0 {
            return;
        }

        self.pos = 0;
        self.update();
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_pos = self.input_byte_pos(self.pos);
        self.input.insert(byte_pos, c);
        self.pos += 1;
        self.update();
    }

    pub fn erase_char(&mut self) {
        let char_len = self.input_char_len();
        let byte_pos = self.input_byte_pos(self.pos);

        if self.pos >= char_len {
            return;
        }

        self.input.remove(byte_pos);
        self.update();
    }

    pub fn backward_erase_char(&mut self) {
        if self.input.is_empty() {
            return;
        }

        let byte_pos = self.input_byte_pos(self.pos);
        // Look at all the bytes that form characters up to the current one,
        // take the last character formed by those bytes and get it's UTF-8
        // length.  If the iterator is empty, it means there are no characters
        // before the current one.
        let prev_char_byte_pos =
            match self.input[..byte_pos].chars().next_back() {
                Some(c) => byte_pos - c.len_utf8(),
                None => return,
            };

        self.input.remove(prev_char_byte_pos);
        self.pos -= 1;
        self.update();
    }

    pub fn backward_erase_line(&mut self) {
        let byte_pos = self.input_byte_pos(self.pos);

        if byte_pos == 0 {
            return;
        }

        self.input.drain(..byte_pos);
        self.pos = 0;
        self.update();
    }

    pub fn history_prev(&mut self) {
        let at_eol = self.pos >= self.input.len();
        self.input = self.history.prev(replace(&mut self.input, String::new()));
        if at_eol {
            self.pos = self.input.len();
        }
        self.update();
    }

    pub fn history_next(&mut self) {
        let at_eol = self.pos >= self.input.len();
        self.input = self.history.next(replace(&mut self.input, String::new()));
        if at_eol {
            self.pos = self.input.len();
        }
        self.update();
    }

    pub fn complete(&mut self) {
        if let Some(text) = &self.completion {
            let byte_pos = self.input_byte_pos(self.pos);
            self.input.insert_str(byte_pos, &text);
            self.pos += text.len();
            self.update();
        }
    }

    fn input_char_len(&self) -> usize {
        self.input.chars().count()
    }

    fn input_byte_pos(&self, pos: usize) -> usize {
        str_byte_pos(&self.input, pos)
    }

    fn update(&mut self) {
        let ParseRes {
            inline_hint,
            completion,
            end_of_line_hint,
            suggestions,
            usage,
            command,
        } = self.commands.parse(&self.input, self.pos);

        self.inline_hint = inline_hint;
        self.completion = completion;
        self.end_of_line_hint = end_of_line_hint;
        self.suggestions = suggestions;
        self.usage = usage;
        self.command = command;
    }
}
