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

use lazy_static::lazy_static;
use num_traits::PrimInt;
use regex::Regex;

use std::fmt::Display;
use std::str::FromStr;

use super::{ArgParseRes, ContextFreeArgParser};

#[derive(PartialEq, Clone, Debug)]
pub struct PrimIntArgParser<T>
where
    T: PrimInt + FromStr + Display,
{
    min: T,
    max: T,
    name: Option<String>,
}

/// The allowed range of integers matches the range of values for the `T` type.
#[cfg(test)]
pub fn prim_int<T>() -> PrimIntArgParser<T>
where
    T: PrimInt + FromStr + Display,
{
    prim_int_for_range(T::min_value(), T::max_value())
}

/// Restricts the allowed range to be `[min, max]` - both end are included.
#[cfg(test)]
pub fn prim_int_for_range<T>(min: T, max: T) -> PrimIntArgParser<T>
where
    T: PrimInt + FromStr + Display,
{
    PrimIntArgParser {
        min,
        max,
        name: None,
    }
}

/// Names the argument.  Name will be included in the hint and in the error
/// messages.
pub fn prim_int_with_name<T, Name>(name: Name) -> PrimIntArgParser<T>
where
    T: PrimInt + FromStr + Display,
    Name: Into<String>,
{
    prim_int_for_range_and_name(T::min_value(), T::max_value(), name)
}

/// Restricts the allowed range to be `[min, max]` - both end are included.
/// Also names the argument.  Name will be included in the hint and in the error
/// messages.
pub fn prim_int_for_range_and_name<T, Name>(
    min: T,
    max: T,
    name: Name,
) -> PrimIntArgParser<T>
where
    T: PrimInt + FromStr + Display,
    Name: Into<String>,
{
    PrimIntArgParser {
        min,
        max,
        name: Some(name.into()),
    }
}

impl<T> ContextFreeArgParser<T> for PrimIntArgParser<T>
where
    T: PrimInt + FromStr + Display,
{
    fn parse(&self, input: &str) -> ArgParseRes<T> {
        lazy_static! {
            static ref NUMBER: Regex = Regex::new(r"^-?\d+$").unwrap();
            static ref NUMBER_PREFIX: Regex = Regex::new(r"^-?\d+").unwrap();
        }

        if !NUMBER.is_match(input) {
            return match NUMBER_PREFIX.find(input) {
                Some(m) => ArgParseRes::Failed {
                    parsed_up_to: m.end(),
                    reason: self.hint(),
                },
                None => ArgParseRes::Failed {
                    parsed_up_to: 0,
                    reason: self.hint(),
                },
            };
        }

        match FromStr::from_str(input) {
            Ok(v) => {
                if v < self.min {
                    ArgParseRes::Failed {
                        parsed_up_to: input.len(),
                        reason: match &self.name {
                            Some(name) => {
                                vec![format!("min {}: {}", name, self.min)]
                            }
                            None => vec![format!("min: {}", self.min)],
                        },
                    }
                } else if v > self.max {
                    ArgParseRes::Failed {
                        parsed_up_to: input.len(),
                        reason: match &self.name {
                            Some(name) => {
                                vec![format!("max {}: {}", name, self.max)]
                            }
                            None => vec![format!("max: {}", self.max)],
                        },
                    }
                } else {
                    ArgParseRes::Parsed(v)
                }
            }
            Err(_) => {
                // `FromStr` errors are very verbose and look strange in our
                // context, so we just return out hint, hoping that the user
                // will guess what is wrong.
                //
                // Calculating `parsed_up_to` is tricky here, as we do not know
                // why the parsing failed.  So we just say that everything
                // parsed as a number.
                ArgParseRes::Failed {
                    parsed_up_to: input.len(),
                    reason: self.hint(),
                }
            }
        }
    }

    fn suggestion(&self, _prefix: &str) -> Vec<String> {
        Vec::new()
    }

    fn hint(&self) -> Vec<String> {
        match &self.name {
            Some(name) => {
                if self.min < T::zero() {
                    vec![format!("<{}: {} - {}>", name, self.min, self.max)]
                } else {
                    vec![format!("<{}: {}-{}>", name, self.min, self.max)]
                }
            }
            None => {
                if self.min < T::zero() {
                    vec![format!("<{} - {}>", self.min, self.max)]
                } else {
                    vec![format!("<{}-{}>", self.min, self.max)]
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        prim_int, prim_int_for_range, prim_int_for_range_and_name,
        prim_int_with_name,
    };

    use crate::input::arg_parser::test_utils::build_cf_parse_checkers;
    use crate::input::arg_parser::ContextFreeArgParser;

    #[test]
    fn u8_parsing() {
        let parser = prim_int::<u8>();
        let expected_hint = &["<0-255>"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("parser", parser);

        check_hint(expected_hint);

        check_parse("0", 0);
        check_parse("1", 1);
        check_parse("255", 255);

        check_failure("-1", 2, expected_hint);
        check_failure("", 0, expected_hint);
        check_failure("a", 0, expected_hint);
        check_failure("z", 0, expected_hint);
        check_failure("*", 0, expected_hint);
        check_failure("256", 3, expected_hint);

        check_suggestions("", &[]);
        check_suggestions("1", &[]);
        check_suggestions("0", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn u8_with_hint() {
        let parser = prim_int_with_name::<u8, _>("width");
        let expected_hint = &["<width: 0-255>"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("parser", parser);

        check_hint(expected_hint);

        check_parse("0", 0);
        check_parse("1", 1);
        check_parse("255", 255);

        check_failure("-1", 2, expected_hint);
        check_failure("", 0, expected_hint);
        check_failure("a", 0, expected_hint);
        check_failure("z", 0, expected_hint);
        check_failure("*", 0, expected_hint);
        check_failure("256", 3, expected_hint);

        check_suggestions("", &[]);
        check_suggestions("1", &[]);
        check_suggestions("0", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn i64_with_range() {
        let i64_arg = prim_int_for_range(-10i64, 1700);
        let expected_below_hint = &["min: -10"];
        let expected_above_hint = &["max: 1700"];
        let expected_hint = &["<-10 - 1700>"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("i64_arg", i64_arg);

        check_hint(expected_hint);

        check_parse("-10", -10);
        check_parse("-7", -7);
        check_parse("0", 0);
        check_parse("1", 1);
        check_parse("1000", 1000);
        check_parse("1700", 1700);

        check_failure("-100", 4, expected_below_hint);
        check_failure("-11", 3, expected_below_hint);
        check_failure("", 0, expected_hint);
        check_failure("a", 0, expected_hint);
        check_failure("z", 0, expected_hint);
        check_failure("*", 0, expected_hint);
        check_failure("1701", 4, expected_above_hint);
        check_failure("100000", 6, expected_above_hint);

        check_suggestions("", &[]);
        check_suggestions("1", &[]);
        check_suggestions("0", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn u64_with_range_and_hint() {
        let u64_arg = prim_int_for_range_and_name(10u64, 100, "height");
        let expected_below_hint = &["min height: 10"];
        let expected_above_hint = &["max height: 100"];
        let expected_hint = &["<height: 10-100>"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("u64_arg", u64_arg);

        check_hint(expected_hint);

        check_parse("10", 10);
        check_parse("17", 17);
        check_parse("100", 100);

        check_failure("-7", 2, expected_hint);
        check_failure("0", 1, expected_below_hint);
        check_failure("3", 1, expected_below_hint);
        check_failure("", 0, expected_hint);
        check_failure("a", 0, expected_hint);
        check_failure("z", 0, expected_hint);
        check_failure("*", 0, expected_hint);
        check_failure("101", 3, expected_above_hint);
        check_failure("100000", 6, expected_above_hint);

        check_suggestions("", &[]);
        check_suggestions("1", &[]);
        check_suggestions("0", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn map() {
        let i8_arg = prim_int::<i8>();

        let saturated_arg = i8_arg.map(|v| if v < 0 { 0 } else { v });

        // Even though we reduce the value set, the input set still matches the
        // `i8` values.
        let expected_hint = &["<-128 - 127>"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("saturated_arg", saturated_arg);

        check_hint(expected_hint);

        check_parse("-10", 0);
        check_parse("0", 0);
        check_parse("33", 33);

        check_failure("-1000", 5, expected_hint);
        check_failure("", 0, expected_hint);
        check_failure("a", 0, expected_hint);
        check_failure("200", 3, expected_hint);

        check_suggestions("", &[]);
        check_suggestions("1", &[]);
        check_suggestions("0", &[]);
        check_suggestions("a", &[]);
    }
}
