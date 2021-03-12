use std::fmt::Debug;
use std::rc::Rc;

use super::{Arg2Parser, ArgParseRes, ContextFreeArgParser};

/// Given a context free parser, generates convenience functions that check the
/// parser `hint()`, `suggestion()` and `parse()` invocations.
pub fn build_cf_parse_checkers<Parser, Res>(
    context: &'static str,
    parser: Parser,
) -> (
    impl for<'a> Fn(&[&'a str]),
    impl for<'a> Fn(&str, &[&'a str]),
    impl Fn(&str, Res),
    impl for<'a> Fn(&str, usize, &[&'a str]),
)
where
    Parser: ContextFreeArgParser<Res>,
    Res: PartialEq + Debug,
{
    let parser = Rc::new(parser);

    let hint = {
        let parser = parser.clone();
        move |hints: &[&str]| {
            let actual = parser.hint();
            let expected = hints
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<String>>();

            assert!(
                actual == expected,
                "{} hint() check failed.\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                expected,
                actual
            );
        }
    };

    let suggestions = {
        let parser = parser.clone();
        move |prefix: &str, suggestions: &[&str]| {
            let actual = parser.suggestion(prefix);
            let expected = suggestions
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<String>>();

            assert!(
                actual == expected,
                "{} suggestion() check failed.\n\
                 prefix:   '{}'\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                prefix,
                expected,
                actual
            );
        }
    };

    let parse_success = {
        let parser = parser.clone();
        move |input: &str, res: Res| {
            let actual = parser.parse(input);
            let expected = ArgParseRes::Parsed(res);
            assert!(
                actual == expected,
                "{} parse() expected success.\n\
                 input:    '{}'\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                input,
                expected,
                actual
            );
        }
    };

    let parse_failure =
        move |input: &str, parsed_up_to: usize, failure: &[&str]| {
            let actual = parser.parse(input);

            let reason = failure.iter().cloned().map(Into::into).collect();
            let expected = ArgParseRes::Failed {
                parsed_up_to,
                reason,
            };

            assert!(
                actual == expected,
                "{} parse() expected failure.\n\
                 input:    '{}'\n\
                 parsed_up_to: {}\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                input,
                parsed_up_to,
                expected,
                actual
            );
        };

    (hint, suggestions, parse_success, parse_failure)
}

/// Given an `arg2` parser, generates convenience functions that check the
/// parser `hint()`, `suggestion()` and `parse()` invocations.
pub fn build_arg2_parse_checkers<Parser, Res1, Res2>(
    context: &'static str,
    parser: Parser,
) -> (
    impl for<'a> Fn(&Res1, &[&'a str]),
    impl for<'a> Fn(&Res1, &str, &[&'a str]),
    impl Fn(&Res1, &str, Res2),
    impl for<'a> Fn(&Res1, &str, usize, &[&'a str]),
)
where
    Parser: Arg2Parser<Res1, Res2>,
    Res1: PartialEq + Debug,
    Res2: PartialEq + Debug,
{
    let parser = Rc::new(parser);

    let hint = {
        let parser = parser.clone();
        move |res1: &Res1, hints: &[&str]| {
            let actual = parser.hint(res1);
            let expected = hints
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<String>>();

            assert!(
                actual == expected,
                "{} hint() check failed.\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                expected,
                actual
            );
        }
    };

    let suggestions = {
        let parser = parser.clone();
        move |res1: &Res1, prefix: &str, suggestions: &[&str]| {
            let actual = parser.suggestion(res1, prefix);
            let expected = suggestions
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<String>>();

            assert!(
                actual == expected,
                "{} suggestion() check failed.\n\
                 prefix:   '{}'\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                prefix,
                expected,
                actual
            );
        }
    };

    let parse_success = {
        let parser = parser.clone();
        move |res1: &Res1, input: &str, res2: Res2| {
            let actual = parser.parse(res1, input);
            let expected = ArgParseRes::Parsed(res2);
            assert!(
                actual == expected,
                "{} parse() expected success.\n\
                 res1:     {:?}\n\
                 input:    '{}'\n\
                 expected: {:?}\n\
                 actual:   {:?}",
                context,
                res1,
                input,
                expected,
                actual
            );
        }
    };

    let parse_failure = move |res1: &Res1,
                              input: &str,
                              parsed_up_to: usize,
                              failure: &[&str]| {
        let actual = parser.parse(res1, input);

        let reason = failure.iter().cloned().map(Into::into).collect();
        let expected = ArgParseRes::Failed {
            parsed_up_to,
            reason,
        };

        assert!(
            actual == expected,
            "{} parse() expected failure.\n\
             res1:     {:?}\n\
             input:    '{}'\n\
             parsed_up_to: {}\n\
             expected: {:?}\n\
             actual:   {:?}",
            context,
            res1,
            input,
            parsed_up_to,
            expected,
            actual
        );
    };

    (hint, suggestions, parse_success, parse_failure)
}
