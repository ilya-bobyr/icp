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

use super::{CommandParseRes, CommandParser, CommandSuggestions};

/// This parser combines several parsers, that all return the same result type,
/// trying them one by one, in order.  It is designed to be used with commands
/// that have several different forms.  The first parser, in order that succeeds
/// is considered to be the result of the `AlternativesCommandParser` parser.
/// In case no parser succeeds the combined parser error is the error generated
/// by the parser that managed to parse the most of the input.
///
/// Suggestions, if any, are combined from all the parsers.
pub struct AlternativesCommandParser<Res> {
    parsers: Vec<Box<dyn CommandParser<Res>>>,
}

pub fn alternatives_cmd<Res, Parsers>(
    parsers: Parsers,
) -> AlternativesCommandParser<Res>
where
    Parsers: IntoIterator<Item = Box<dyn CommandParser<Res>>>,
{
    let parsers = parsers.into_iter().collect::<Vec<_>>();

    if parsers.is_empty() {
        panic!("`parsers` should not be empty");
    }

    AlternativesCommandParser { parsers }
}

impl<Res> CommandParser<Res> for AlternativesCommandParser<Res> {
    fn parse(
        &self,
        input: &str,
        pos: Option<usize>,
    ) -> (CommandParseRes<Res>, Option<CommandSuggestions>) {
        let mut parsers = self.parsers.iter();

        let (mut combined_res, mut combined_suggestions) = {
            // `self.parsers` must be non-empty.
            let parser = parsers.next().unwrap();

            parser.parse(input, pos)
        };

        for parser in parsers {
            let (res, suggestions) = parser.parse(input, pos);

            combined_res = combined_res.merge(res);

            combined_suggestions = match (combined_suggestions, suggestions) {
                (None, suggestions) => suggestions,
                (combined_suggestions @ Some(_), None) => combined_suggestions,
                (Some(mut combined_suggestions), Some(mut suggestions)) => {
                    combined_suggestions.0.append(&mut suggestions.0);
                    Some(combined_suggestions)
                }
            }
        }

        (combined_res, combined_suggestions)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::input::arg_parser::prim_int_for_range;
    use crate::input::arg_parser::ContextFreeArgParser;
    use crate::input::arg_parser::{keyword_set, keyword_set_with_hint};
    use crate::input::command_parser::test_utils::check_parse_and_failure_generator;
    use crate::input::command_parser::{
        command_1arg, command_2args, CommandParseFailure,
    };

    use super::{alternatives_cmd, CommandParser, CommandSuggestions};

    macro_rules! vec_str {
        ($( $ex:expr ),* $(,)*) => {
            vec![ $( $ex.to_string() ),* ]
        };
    }

    #[test]
    fn simple_alternatives_parser() {
        #[derive(PartialEq, Clone, Debug)]
        enum TestCommand {
            East(u8),
            West(u8),
            Reset,
        };

        let parser = {
            let opt1_parser = {
                let arg1 =
                    keyword_set_with_hint(&["east", "west"], &["<side>"]);
                let arg2 = prim_int_for_range(0u8, 63);

                command_2args(arg1, arg2.adapt(), |dir, x| match dir.as_str() {
                    "east" => TestCommand::East(x),
                    "west" => TestCommand::West(x),
                    _ => panic!("Unexpected keyword: {}", dir),
                })
                .boxed()
            };

            let opt2_parser = {
                let arg1 = keyword_set(&["reset"]);
                command_1arg(arg1, |_| TestCommand::Reset).boxed()
            };

            alternatives_cmd(vec![opt1_parser, opt2_parser])
        };

        let (check_parse, check_failure) =
            check_parse_and_failure_generator(parser);

        use CommandParseFailure::{
            ArgumentParseFailed, ExpectedArg, UnexpectedArgument,
        };

        // == ExpectedArg ==

        check_failure(
            "",
            Some(0),
            0,
            ExpectedArg {
                index: 0,
                hint: vec_str!["<side>", "reset"],
            },
            Some(CommandSuggestions(vec_str!["east", "west", "reset"])),
        );

        // == Pared ==

        for cur in 1..3 {
            check_parse(
                "east 7",
                Some(cur),
                TestCommand::East(7),
                Some(CommandSuggestions(vec_str!["east"])),
            );
        }

        check_parse(
            "east 7",
            Some(4),
            TestCommand::East(7),
            Some(CommandSuggestions(vec![])),
        );
        check_parse(
            "east 7",
            Some(5),
            TestCommand::East(7),
            Some(CommandSuggestions(vec![])),
        );

        // == UnexpectedArgument ==

        check_failure(
            "east 7 more",
            Some(6),
            6,
            UnexpectedArgument { from: 7 },
            Some(CommandSuggestions(vec![])),
        );

        for cur in 7..11 {
            check_failure(
                "east 7 more",
                Some(cur),
                6,
                UnexpectedArgument { from: 7 },
                None,
            );
        }

        // == ArgumentParseFailed ==

        check_failure(
            "ea",
            Some(0),
            2,
            ArgumentParseFailed {
                from: 0,
                to: 2,
                reason: vec_str!["<side>"],
            },
            Some(CommandSuggestions(vec_str!["east", "west", "reset"])),
        );
        for cur in 1..2 {
            check_failure(
                "ea",
                Some(cur),
                2,
                ArgumentParseFailed {
                    from: 0,
                    to: 2,
                    reason: vec_str!["<side>"],
                },
                Some(CommandSuggestions(vec_str!["east"])),
            );
        }
        check_failure(
            "ea",
            Some(3),
            2,
            ArgumentParseFailed {
                from: 0,
                to: 2,
                reason: vec_str!["<side>"],
            },
            None,
        );
    }
}
