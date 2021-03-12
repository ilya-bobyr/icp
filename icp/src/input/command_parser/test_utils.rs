//! Common code for testing.

use std::fmt::Debug;
use std::rc::Rc;

use super::{CommandParseRes, CommandParser, CommandSuggestions};

use crate::input::command_parser::CommandParseFailure;

/// Given a parser, generates convenience functions that assert on either
/// success or failure of parsing of a specific input.
pub fn check_parse_and_failure_generator<Parser, Res>(
    parser: Parser,
) -> (
    impl Fn(&str, Option<usize>, Res, Option<CommandSuggestions>),
    impl Fn(
        &str,
        Option<usize>,
        usize,
        CommandParseFailure,
        Option<CommandSuggestions>,
    ),
)
where
    Parser: CommandParser<Res>,
    Res: PartialEq + Debug,
{
    let parser = Rc::new(parser);

    let success = {
        let parser = parser.clone();
        move |input: &str,
              pos: Option<usize>,
              parse: Res,
              suggestions: Option<CommandSuggestions>| {
            let actual = parser.parse(input, pos);
            let expected = (CommandParseRes::Parsed(parse), suggestions);
            assert!(
                actual == expected,
                "`check_parse` failed for: '{}', pos: {:?}\n\
                 Expected: {:?}\n\
                 Actual:   {:?}",
                input,
                pos,
                expected,
                actual
            );
        }
    };

    let failure =
        move |input: &str,
              pos: Option<usize>,
              parsed_up_to: usize,
              reason: CommandParseFailure,
              suggestions: Option<CommandSuggestions>| {
            let actual = parser.parse(input, pos);
            let expected = (
                CommandParseRes::Failed {
                    parsed_up_to,
                    reason,
                },
                suggestions,
            );
            assert!(
                actual == expected,
                "`check_failure` failed for: '{}', pos: {:?}\n\
                 Expected: {:?}\n\
                 Actual:   {:?}",
                input,
                pos,
                expected,
                actual
            );
        };

    (success, failure)
}
