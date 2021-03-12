//! Command are objects produced from the user input.
//!
//! This module contains command parsing mechanism that is responsible for
//! parsing commands, showing usage information and providing completion
//! suggestions, making it easier for the user to discover and use commands.
//!
//! There is also a predefined [`help`] command, and a [`table`] of commands,
//! that holds all the commands available to the user.
//!
//! Currently, commands are distinguished by the first word the user enters.
//! Completion and suggestions work differently when entering the first word
//! into the command prompt vs entering arguments for a specific command.
//!
//! While entering the first word, it is compared against the `keyword()` values
//! of all the registered commands.  At the moment, the full command name need
//! to be type, but suggestions will show all the possibilities and completion
//! can be used to speed things up a bit.  It would make sense to accept a
//! non-ambiguous prefix in place of a full command keyword.
//!
//! When the user types the first white space a specific command is selected.
//! At this point completion is based on the selected command and the
//! suggestions show possible values for this particular command.

pub mod table;

pub mod help;

pub use table::CommandsTable;

use std::fmt;

use crate::input::command_parser::{CommandParseRes, CommandSuggestions};

/// An end of line hint may refer either to the whole input or to a specific
/// subsection.  See [`EndOfLineHint`].
#[derive(PartialEq, Clone, Debug)]
pub enum EndOfLineHintTarget {
    WholeLine,
    Substring { from: usize, to: usize },
}

/// See usage in [`input::Input`] for details.
#[derive(PartialEq, Clone, Debug)]
pub enum HintType {
    Info,
    Error,
}

#[derive(PartialEq, Clone, Debug)]
pub struct EndOfLineHint {
    pub target: EndOfLineHintTarget,
    pub type_: HintType,
    pub text: String,
}

/// Result of a [`CommandsTable::parse()`] method.  Returned values are stored
/// in the corresponding fields of the [`input::Input`] struct.  See there for
/// the details about all the fields.
pub struct ParseRes {
    pub inline_hint: Option<String>,
    pub completion: Option<String>,
    pub end_of_line_hint: Option<EndOfLineHint>,
    pub suggestions: Vec<String>,
    pub usage: Option<String>,
    pub command: Option<Box<dyn Executor>>,
}

impl fmt::Debug for ParseRes {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("ParseRes")
            .field("inline_hint", &self.inline_hint)
            .field("completion", &self.completion)
            .field("end_of_line_hint", &self.end_of_line_hint)
            .field("suggestions", &self.suggestions)
            .field("usage", &self.usage)
            .field(
                "command",
                if self.command.is_some() {
                    &"Some(_)"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

/// Every command is described by an instance of this type.
pub trait Command {
    /// Keyword names this command.  When the user is typing a command, they
    /// need to type this string to select this particular command.
    fn keyword(&self) -> &str;

    /// One line help string.  To be shown to the user when they are typing the
    /// command.
    fn short_usage(&self) -> &str;

    /// Multi line help string.  To be shown in the command help message.
    fn long_usage(&self) -> &str;

    /// Parses command arguments.  Returns either a failure with a detailed
    /// explanation as to why the parsing failed or an object that stores the
    /// command arguments in a ready-to-run form.
    ///
    /// In addition, provides possible completions at the specified character
    /// position.
    fn parse(
        &self,
        input: &str,
        pos: Option<usize>,
    ) -> (
        CommandParseRes<Box<dyn Executor>>,
        Option<CommandSuggestions>,
    );
}

/// When command is parsed its arguments are stored in a parsed form inside an
/// object that implements this trait, allowing the command to be run.
///
// TODO: When trait aliases are stabilized, this should be just a trait alias
// instead.  See https://github.com/rust-lang/rust/issues/41517.
pub trait Executor: FnOnce() {
    /// We almost always need to box `Executor` closures, as we are passing them
    /// as a result of a parse operation.  This method removes some of the
    /// syntactic noise from the closer construction site.
    fn boxed(self) -> Box<dyn Executor>
    where
        Self: Sized + 'static,
    {
        Box::new(self) as Box<dyn Executor>
    }
}

impl<T> Executor for T where T: FnOnce() {}
