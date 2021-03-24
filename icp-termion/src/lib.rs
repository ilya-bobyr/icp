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

//! A termion specific implementation of an input prompt.  Wraps
//! [`crate::Input`], providing an implementation that connects `Input` to an
//! actual terminal input and output.

use std::borrow::Cow;
use std::io::{self, Write};

use termion::event::{Event, Key};
use termion::{self, color, cursor};

use icp::commands::table::CommandsTable;
use icp::commands::EndOfLineHint;
use icp::{self, Prompt};
use icp::{str_byte_pos, TerminalContentRef};

/// Wraps an [`icp::Input`] instance, providing visual representation on a
/// given terminal.
pub struct Input<Terminal>
where
    Terminal: TerminalContentRef,
{
    inner: icp::Input,
    terminal: Terminal,
}

impl<Terminal> Input<Terminal>
where
    Terminal: TerminalContentRef + 'static,
{
    pub fn new(
        prompt: Prompt,
        commands: CommandsTable,
        terminal: Terminal,
    ) -> Self {
        Self {
            inner: icp::Input::new(prompt, commands),
            terminal,
        }
    }

    pub fn input(&mut self, event: termion::event::Event) {
        let inner = &mut self.inner;
        match event {
            Event::Key(Key::Char('\n')) =>
            {
                self.execute();
            }
            Event::Key(Key::Up) | Event::Key(Key::Ctrl('p')) => {
                inner.history_prev();
            }
            Event::Key(Key::Down) | Event::Key(Key::Ctrl('n')) => {
                inner.history_next();
            }
            Event::Key(Key::Left) | Event::Key(Key::Ctrl('b')) => {
                inner.cursor_left();
            }
            Event::Key(Key::Right) | Event::Key(Key::Ctrl('f')) => {
                inner.cursor_right();
            }
            Event::Key(Key::Char('\t')) => {
                inner.complete();
            }
            Event::Key(Key::Ctrl('a')) => {
                inner.cursor_start();
            }
            Event::Key(Key::Ctrl('e')) => {
                inner.cursor_end();
            }
            Event::Key(Key::Backspace) | Event::Key(Key::Ctrl('h')) => {
                inner.backward_erase_char();
            }
            Event::Key(Key::Delete) | Event::Key(Key::Ctrl('d')) => {
                inner.erase_char();
            }
            Event::Key(Key::Ctrl('u')) => {
                inner.backward_erase_line();
            }
            Event::Key(Key::Char(c))
                // Do not add "control" characters into the input buffer.  There
                // is unlikely a use case where they would be useful.  But they
                // cause a lot of trouble as they may mess up with the output as
                // we print the input buffer to the terminal literally.  In the
                // future we should probably use `linefeed`[1] or a similar
                // library to provide reach user experience for text input.
                //
                // [1]: https://crates.io/crates/linefeed
                if !c.is_control() =>
            {
                inner.insert_char(c);
            }
            _ => (),
        }
    }

    /// Draw the input area and the suggestions area at the specified
    /// `(x, y)` coordinates, all the way to the right edge of the terminal.
    /// Currently takes 2 lines.
    pub fn draw(
        &self,
        x: u16,
        y: u16,
        screen: &mut dyn Write,
        max_width: u16,
    ) -> io::Result<()> {
        // TODO: There is a number of color constants in this method body.  I
        // expect them to be moved into a "color scheme" object, where they
        // would have structure and names.  While experimenting with the layout
        // and colors it is convenient to have them "hardcoded" in the places
        // where they are used.

        let max_width = max_width as usize;

        write!(screen, "{}", termion::style::Reset)?;

        write!(
            screen,
            "{}{}",
            cursor::Goto(x, y),
            color::Bg(color::Rgb(0, 43, 54))
        )?;

        let prompt_len;

        let inner = &self.inner;

        if inner.input().is_empty() {
            write!(
                screen,
                "{}{}",
                color::Fg(color::Rgb(0, 95, 255)),
                &inner.prompt().empty,
            )?;
            prompt_len = inner.prompt().empty.chars().count();
        } else if inner.command().is_none() {
            if inner.suggestions().is_empty() {
                write!(
                    screen,
                    "{}{}",
                    color::Fg(color::Rgb(215, 95, 0)),
                    &inner.prompt().invalid,
                )?;
                prompt_len = inner.prompt().invalid.chars().count();
            } else {
                write!(
                    screen,
                    "{}{}",
                    color::Fg(color::Rgb(0, 95, 255)),
                    &inner.prompt().incomplete,
                )?;
                prompt_len = inner.prompt().incomplete.chars().count();
            }
        } else {
            write!(
                screen,
                "{}{}",
                color::Fg(color::Rgb(95, 175, 0)),
                &inner.prompt().complete,
            )?;
            prompt_len = inner.prompt().complete.chars().count();
        };

        write!(
            screen,
            "{}{}",
            color::Fg(color::Rgb(129, 158, 150)),
            &inner.input().chars().take(inner.pos()).collect::<String>(),
        )?;

        write!(screen, "{}", cursor::Save)?;

        if let Some(hint) = &inner.inline_hint() {
            write!(screen, "{}{}", color::Fg(color::Rgb(38, 139, 210)), hint,)?;
        }

        write!(
            screen,
            "{}{}",
            color::Fg(color::Rgb(129, 158, 150)),
            &inner.input().chars().skip(inner.pos()).collect::<String>(),
        )?;

        if let Some(EndOfLineHint { text, .. }) = &inner.end_of_line_hint() {
            let input_len = inner.input().chars().count();
            let inline_hint_len = self
                .inner
                .inline_hint()
                .map(|hint| hint.chars().count())
                .unwrap_or(0);

            let chars_left = max_width
                .saturating_sub(prompt_len)
                .saturating_sub(input_len)
                .saturating_sub(inline_hint_len)
                .saturating_sub(2 /* indent */);
            write!(
                screen,
                "  {}{}",
                color::Fg(color::Rgb(178, 122, 26)),
                text_limit_width(text, chars_left)
            )?;
        }

        write!(
            screen,
            "{}{}{}",
            termion::clear::UntilNewline,
            cursor::Goto(x, y + 1),
            color::Bg(color::Rgb(0, 43, 54)),
        )?;

        if !inner.suggestions().is_empty() {
            let suggestions = inner.suggestions().join("  ");

            write!(
                screen,
                "  {}{}",
                color::Fg(color::Rgb(181, 137, 0)),
                text_limit_width(&suggestions, max_width.saturating_sub(3)),
            )?;
        }

        write!(screen, "{}", termion::clear::UntilNewline)?;

        Ok(())
    }

    pub fn execute(&mut self) {
        let inner = &mut self.inner;
        if inner.command().is_some() {
            self.terminal.push(format!(
                "{}{}{}{}{}{}{}",
                color::Bg(color::Rgb(0, 43, 54)),
                termion::clear::UntilNewline,
                color::Fg(color::Rgb(0, 95, 255)),
                inner.prompt().complete,
                color::Fg(color::Rgb(129, 158, 150)),
                inner.input(),
                termion::clear::UntilNewline,
            ));
            inner.execute();
        }
    }
}

/// Makes sure that a string does not exceed the specified width.  If it
/// does, cuts the string to make it fit, adding ' ...' at the end.
fn text_limit_width(text: &str, max_width: usize) -> Cow<str> {
    let text_len = text.chars().count();
    let ellipsis = " ...";

    if text_len <= max_width {
        Cow::from(text)
    } else if max_width < 2 * ellipsis.len() {
        // It does not make sense to insert ellipsis if there is less space than
        // the space the ellipsis will take themselves, so we just cut in this
        // case.
        let up_to = str_byte_pos(text, max_width);
        Cow::from(&text[0..up_to])
    } else {
        let up_to = str_byte_pos(text, max_width - ellipsis.len());
        Cow::from(text[0..up_to].to_string() + ellipsis)
    }
}
