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

//! A table of all the commands supported by PET.

use indoc::indoc;
use lazy_static::lazy_static;
use regex::Regex;

use std::iter::once;
use std::ops::Deref;
use std::rc::{Rc, Weak};

use crate::input::command_parser::{CommandParseFailure, CommandParseRes};
use crate::input::common_prefix;
use crate::TerminalContentRef;

use super::{
    help, Command, EndOfLineHint, EndOfLineHintTarget, HintType, ParseRes,
};

pub struct CommandsTable(Rc<Vec<Box<dyn Command>>>);

/// A "weak" reference to a `CommandsTable`.  `CommandsTable` internally uses an
/// `Rc`, and this is an [`std::rc::Weak`] counterpart to it.
pub struct CommandsTableWeak(Weak<Vec<Box<dyn Command>>>);

static HELP_MSG: &str = indoc!(
    r"
    Chip Debugging Tool

    Function key shortcuts are along the bottom of the screen.

    Commands:
"
);

impl CommandsTable {
    pub fn new(
        terminal: impl TerminalContentRef + 'static,
        commands: impl Iterator<Item = Box<dyn Command + 'static>>,
    ) -> Self {
        let (help_cmd, help_initializer) = help::command(terminal);

        let commands =
            CommandsTable(Rc::new(commands.chain(once(help_cmd)).collect()));

        (help_initializer)(commands.clone());

        commands
    }

    pub fn default_usage(&self) -> String {
        HELP_MSG.to_string()
            + &help::all_commands_usage(self.clone()).join("\n")
    }

    pub fn downgrade(&self) -> CommandsTableWeak {
        CommandsTableWeak(Rc::downgrade(&self.0))
    }

    /// Similar to [`Command::parse`].  Parses user `input`, interpreting it as
    /// one of the commands stored in this table.  `pos` is the character for
    /// which the suggestions are generated - essentially it would be the cursor
    /// position in the UI.
    pub fn parse(&self, input: &str, pos: usize) -> ParseRes {
        lazy_static! {
            static ref COMMAND: Regex = Regex::new(r"\s*(\S+)\s*(.*)").unwrap();
        }

        let caps = match COMMAND.captures(input) {
            None => return empty_input(self.0.as_ref()),
            Some(caps) => caps,
        };

        let input_command = caps.get(1).unwrap();
        let args = caps.get(2).unwrap();

        if let Some(command) = self
            .0
            .as_ref()
            .iter()
            .find(|c| c.keyword() == input_command.as_str())
        {
            let start = args.start();
            let end = args.end();
            let pos = if pos >= start && pos <= end {
                Some(pos - start)
            } else {
                None
            };
            return parse_args(
                command.as_ref(),
                args.as_str(),
                pos,
                args.end(),
            );
        }

        let matching = self
            .0
            .as_ref()
            .iter()
            .filter(|c| c.keyword().starts_with(input_command.as_str()))
            .map(|c| c.as_ref())
            .collect::<Vec<_>>();

        if !matching.is_empty() {
            let start = input_command.start();
            let end = input_command.end();
            if pos < start || end < pos {
                return prefix_command_no_hints();
            }

            let prefix = &input_command.as_str()[0..pos - start];

            return prefix_command(prefix, &matching);
        }

        no_match()
    }
}

fn empty_input(commands: &[Box<dyn Command>]) -> ParseRes {
    ParseRes {
        inline_hint: Some("<command>".to_string()),
        completion: None,
        end_of_line_hint: None,
        suggestions: commands
            .iter()
            .map(|k| k.keyword().to_string())
            .collect::<Vec<_>>(),
        usage: Some("Waiting for a command".to_string()),
        command: None,
    }
}

fn no_match() -> ParseRes {
    ParseRes {
        inline_hint: None,
        completion: None,
        end_of_line_hint: Some(EndOfLineHint {
            target: EndOfLineHintTarget::WholeLine,
            type_: HintType::Error,
            text: "TODO no_match".to_string(),
        }),
        suggestions: vec![],
        usage: Some("TODO: usage".to_string()),
        command: None,
    }
}

fn prefix_command_no_hints() -> ParseRes {
    ParseRes {
        inline_hint: None,
        completion: None,
        end_of_line_hint: Some(EndOfLineHint {
            target: EndOfLineHintTarget::WholeLine,
            type_: HintType::Info,
            text: "TODO prefix_command_no_hints".to_string(),
        }),
        suggestions: vec![],
        usage: Some("TODO: prefix_command_no_hints usage".to_string()),
        command: None,
    }
}

fn hint_and_completion<'a>(
    prefix: &str,
    suggestions: impl Iterator<Item = &'a str>,
) -> (Option<String>, Option<String>) {
    let common = common_prefix(suggestions);

    if common.is_empty() || common.len() == prefix.len() {
        (None, None)
    } else if common.len() == 1 {
        let inline_hint = common[prefix.len()..].to_string();
        let mut completion = inline_hint.clone();
        // As there is only one match we can produce the whitespace as well.
        completion.push(' ');
        (Some(inline_hint), Some(completion))
    } else {
        let value = common[prefix.len()..].to_string();
        (Some(value.clone()), Some(value))
    }
}

fn prefix_command(prefix: &str, commands: &[&dyn Command]) -> ParseRes {
    let (inline_hint, completion) =
        hint_and_completion(prefix, commands.iter().map(|c| c.keyword()));

    let suggestions = commands
        .iter()
        .map(|c| c.keyword().to_string())
        .collect::<Vec<_>>();

    ParseRes {
        inline_hint,
        completion,
        end_of_line_hint: Some(EndOfLineHint {
            target: EndOfLineHintTarget::WholeLine,
            type_: HintType::Info,
            text: "<command>".to_string(),
        }),
        suggestions,
        usage: Some("TODO: prefix_command usage".to_string()),
        command: None,
    }
}

fn parse_args(
    command: &dyn Command,
    args: &str,
    pos: Option<usize>,
    args_end: usize,
) -> ParseRes {
    use CommandParseFailure::{
        ArgumentParseFailed, ExpectedArg, UnexpectedArgument,
    };

    match command.parse(args, pos) {
        (CommandParseRes::Parsed(exec), suggestions) => ParseRes {
            inline_hint: None,
            completion: None,
            end_of_line_hint: None,
            suggestions: suggestions
                .map(Into::<Vec<String>>::into)
                .unwrap_or_default(),
            usage: Some("TODO: parse_args usage".to_string()),
            command: Some(exec),
        },
        (
            CommandParseRes::Failed {
                parsed_up_to: _,
                reason: ArgumentParseFailed { from, to, reason },
            },
            suggestions,
        ) => ParseRes {
            inline_hint: None,
            completion: None,
            end_of_line_hint: Some(EndOfLineHint {
                target: EndOfLineHintTarget::Substring { from, to },
                type_: HintType::Error,
                text: reason.join(" | "),
            }),
            suggestions: suggestions.map(Into::into).unwrap_or_default(),
            usage: Some("TODO: parse_args usage".to_string()),
            command: None,
        },
        (
            CommandParseRes::Failed {
                parsed_up_to: _,
                reason: ExpectedArg { index: _, hint },
            },
            suggestions,
        ) => ParseRes {
            inline_hint: None,
            completion: None,
            end_of_line_hint: Some(EndOfLineHint {
                target: EndOfLineHintTarget::WholeLine,
                type_: HintType::Error,
                text: hint.join(" | "),
            }),
            suggestions: suggestions.map(Into::into).unwrap_or_default(),
            usage: Some("TODO: parse_args usage".to_string()),
            command: None,
        },
        (
            CommandParseRes::Failed {
                parsed_up_to: _,
                reason: UnexpectedArgument { from },
            },
            suggestions,
        ) => ParseRes {
            inline_hint: None,
            completion: None,
            end_of_line_hint: Some(EndOfLineHint {
                target: EndOfLineHintTarget::Substring { from, to: args_end },
                type_: HintType::Error,
                text: "Unexpected argument".to_string(),
            }),
            suggestions: suggestions.map(Into::into).unwrap_or_default(),
            usage: Some("TODO: parse_args usage".to_string()),
            command: None,
        },
    }
}

impl Deref for CommandsTable {
    type Target = Vec<Box<dyn Command>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for CommandsTable {
    fn clone(&self) -> Self {
        CommandsTable(self.0.clone())
    }
}

impl CommandsTableWeak {
    pub fn upgrade(&self) -> Option<CommandsTable> {
        self.0.upgrade().map(CommandsTable)
    }
}
