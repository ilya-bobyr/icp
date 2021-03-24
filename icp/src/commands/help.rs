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

//! "help" command.

use indoc::indoc;

use std::cell::RefCell;
use std::cmp::max;
use std::rc::Rc;

use crate::input::arg_parser::keyword_set_with_hint;
use crate::input::command_parser::alternatives::AlternativesCommandParser;
use crate::input::command_parser::{
    alternatives_cmd, command_1arg, command_no_args, CommandParser,
};
use crate::TerminalContentRef;

use super::table::{CommandsTable, CommandsTableWeak};
use super::{Command, CommandParseRes, CommandSuggestions, Executor};

/// Returns the `Help` command and an initialization function that needs to be
/// called after a [`CommandsTable`] instance holding this `Help` instance is
/// constructed.  This way the `Help` instance will have a reference to the
/// parent [`CommandsTable`] instance, allowing it to access the full list of
/// available commands.
pub fn command(
    terminal: impl TerminalContentRef + 'static,
) -> (Box<dyn Command>, impl Fn(CommandsTable)) {
    Help::new(terminal)
}

struct Help {
    inner: Rc<RefCell<Inner>>,
}

enum Inner {
    /// This is the state of the `Help` object immediately after it is
    /// constructed, until the closure that inserts a connection to the
    /// `CommandsTable` is invoked.  See [`new()`] for details.
    Uninitialized,

    /// This is the "normal" state of the `Help` instance.  After it has been
    /// connected to the `CommandsTable` that holds the list of all the
    /// available commands.
    Initialized {
        parser: AlternativesCommandParser<Box<dyn Executor>>,

        /// A reference to the commands table needs to be a "weak" one, as the
        /// commands table also references the help command itself.  So a strong
        /// reference would create a cycle.
        commands: CommandsTableWeak,
    },
}

impl Help {
    /// An instance of `Help` needs a reference to the `CommandsTable`.  But the
    /// `CommandsTable` needs a full list of all the commands when it is
    /// constructed.  There is a loop here, that is broken by allowing `Help` to
    /// have an `Uninitialized` state.  `new()` returns a `Help` instance that
    /// is uninitialized and a closure that needs to be called with a
    /// `CommandsTable` instance holding this `Help` instance to finish the
    /// loop.
    #[allow(clippy::new_ret_no_self)]
    fn new(
        terminal: impl TerminalContentRef + 'static,
    ) -> (Box<dyn Command>, impl Fn(CommandsTable)) {
        let inner = Rc::new(RefCell::new(Inner::Uninitialized));

        (
            {
                let inner = inner.clone();
                Box::new(Help { inner }) as Box<dyn Command>
            },
            Self::set_commands(inner, terminal),
        )
    }

    fn set_commands(
        inner: Rc<RefCell<Inner>>,
        terminal: impl TerminalContentRef + 'static,
    ) -> impl Fn(CommandsTable) {
        move |table: CommandsTable| {
            let for_all = {
                let inner = inner.clone();
                let terminal = terminal.clone();
                command_no_args(move || {
                    let inner = inner.clone();
                    let terminal = terminal.clone();
                    (move || {
                        inner.borrow().help_for_all(terminal);
                    })
                    .boxed()
                })
                .boxed()
            };

            let specific = {
                let inner = inner.clone();
                let terminal = terminal.clone();

                let arg1 = keyword_set_with_hint(
                    table.iter().map(|c| c.keyword()),
                    &["<command name>"],
                );

                command_1arg(arg1, move |keyword| {
                    let inner = inner.clone();
                    let terminal = terminal.clone();
                    (move || {
                        inner.borrow().help_for(&keyword, terminal);
                    })
                    .boxed()
                })
                .boxed()
            };

            let parser = alternatives_cmd(vec![for_all, specific]);

            *inner.borrow_mut() = Inner::Initialized {
                parser,
                commands: table.downgrade(),
            };
        }
    }
}

impl Command for Help {
    fn keyword(&self) -> &str {
        "help"
    }

    fn short_usage(&self) -> &str {
        "All the commands and their descriptions."
    }

    fn long_usage(&self) -> &str {
        indoc!(
            r"
            help

                Shows the list of all the supported commands along with their
                descriptions.

            help <command>

                Show detailed description of the specified command.
        "
        )
    }

    fn parse(
        &self,
        input: &str,
        pos: Option<usize>,
    ) -> (
        CommandParseRes<Box<dyn Executor>>,
        Option<CommandSuggestions>,
    ) {
        match &*self.inner.borrow() {
            Inner::Uninitialized => {
                panic!("`parse` called before `set_commands()` was called")
            }
            Inner::Initialized { parser, .. } => parser.parse(input, pos),
        }
    }
}

impl Inner {
    fn for_commands(&self, run: impl FnOnce(CommandsTable)) {
        match self {
            Inner::Uninitialized => panic!(
                "`for_commands` called before `set_commands()` was called"
            ),
            Inner::Initialized { commands, .. } => match commands.upgrade() {
                Some(commands) => run(commands),
                None => panic!(
                    "`for_commands` invoked after the commands table is \
                     already gone"
                ),
            },
        }
    }

    fn help_for_all(&self, mut terminal: impl TerminalContentRef) {
        self.for_commands(|commands| {
            terminal.extend(all_commands_usage(commands).into_iter())
        });
    }

    fn help_for(&self, keyword: &str, mut terminal: impl TerminalContentRef) {
        self.for_commands(|commands| {
            if let Some(command) =
                commands.iter().find(|c| c.keyword() == keyword)
            {
                terminal.extend(
                    command.long_usage().lines().map(ToString::to_string),
                );
            } else {
                debug_assert!(false,
                    "`help_for` called with keyword that is not a keyword of a \
                     registered command.\n\
                     Keyword: '{}'",
                    keyword
                );
            }
        });
    }
}

pub fn all_commands_usage(table: CommandsTable) -> Vec<String> {
    let max_width = table.iter().map(|c| c.keyword().len()).fold(0, max);

    table
        .iter()
        .map(|command| {
            format!(
                "  {keyword:max_width$}    {short_usage}",
                keyword = command.keyword(),
                max_width = max_width,
                short_usage = command.short_usage(),
            )
        })
        .collect()
}
