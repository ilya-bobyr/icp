use super::{Arg2Parser, ArgParseRes, ContextFreeArgParser};

use std::marker::PhantomData;

/// This parser runs another parser and applies a function to the value it
/// produces, converting the return type.  Suggestions and hints are just passed
/// as is.
///
/// You should use a `map` method on the parser, instead of using this type
/// directly.
pub struct ContextFreeMap<A, B, Parser, F>
where
    Parser: ContextFreeArgParser<A>,
    F: Fn(A) -> B,
{
    parser: Parser,
    f: F,
    _a: PhantomData<fn(A)>,
    _b: PhantomData<B>,
}

impl<A, B, Parser, F> ContextFreeMap<A, B, Parser, F>
where
    Parser: ContextFreeArgParser<A>,
    F: Fn(A) -> B,
{
    pub fn new(parser: Parser, f: F) -> Self {
        Self {
            parser,
            f,
            _a: PhantomData,
            _b: PhantomData,
        }
    }
}

impl<A, B, Parser, F> ContextFreeArgParser<B>
    for ContextFreeMap<A, B, Parser, F>
where
    Parser: ContextFreeArgParser<A>,
    F: Fn(A) -> B,
{
    fn parse(&self, input: &str) -> ArgParseRes<B> {
        match self.parser.parse(input) {
            ArgParseRes::Failed {
                parsed_up_to,
                reason,
            } => ArgParseRes::Failed {
                parsed_up_to,
                reason,
            },
            ArgParseRes::Parsed(res) => ArgParseRes::Parsed((self.f)(res)),
        }
    }

    fn suggestion(&self, prefix: &str) -> Vec<String> {
        self.parser.suggestion(prefix)
    }

    fn hint(&self) -> Vec<String> {
        self.parser.hint()
    }
}

/// Generates "context-sensitive" argument parser that maps another parser -
/// similar to [`ContextFreeMap`] but for [`Arg2Parser`] and friends.  You can
/// use [`Arg2ContextFreeAdapter`] if you need to use a context free argument
/// parser as part of an alternative that is used for a non-context free
/// argument.
///
/// You should use a `map` method on the parser, instead of using this type
/// directly.
macro_rules! define_arg_parser_map {
    (
        $name:ident: $parser_trait:ident,
        { $( $arg_name:ident: $arg_type:ident ($phantom_name:ident) ),* $(,)* },
        $res1:ident, $res2:ident
    ) => {
        pub struct $name<$( $arg_type, )* $res1, $res2, Parser, F>
        where
            Parser: $parser_trait<$( $arg_type, )* $res1>,
            F: Fn($( &$arg_type, )* $res1) -> $res2,
        {
            parser: Parser,
            f: F,
            $( $phantom_name: PhantomData<$arg_type>, )*
            _a: PhantomData<fn($res1)>,
            _b: PhantomData<$res2>,
        }

        impl<$( $arg_type, )* $res1, $res2, Parser, F>
            $name<$( $arg_type, )* $res1, $res2, Parser, F>
        where
            Parser: $parser_trait<$( $arg_type, )* $res1>,
            F: Fn($( &$arg_type, )* $res1) -> $res2,
        {
            #[allow(unused)]
            pub fn new(parser: Parser, f: F) -> Self
            {
                Self {
                    parser,
                    f,
                    $( $phantom_name: PhantomData, )*
                    _a: PhantomData,
                    _b: PhantomData,
                }
            }
        }

        impl<$( $arg_type, )* $res1, $res2, Parser, F>
            $parser_trait<$( $arg_type, )* $res2>
            for $name<$( $arg_type, )* $res1, $res2, Parser, F>
        where
            Parser: $parser_trait<$( $arg_type, )* $res1>,
            F: Fn($( &$arg_type, )* $res1) -> $res2,
        {
            fn parse(&self, $( $arg_name: &$arg_type, )* input: &str)
                -> ArgParseRes<$res2>
            {
                match self.parser.parse($( $arg_name, )* input) {
                    ArgParseRes::Failed { parsed_up_to, reason } =>
                        ArgParseRes::Failed { parsed_up_to, reason },
                    ArgParseRes::Parsed(res) => {
                        let res = (self.f)($( $arg_name, )* res);
                        ArgParseRes::Parsed(res)
                    }
                }
            }

            fn suggestion(&self, $( $arg_name: &$arg_type, )* prefix: &str)
                -> Vec<String>
            {
                self.parser.suggestion($( $arg_name, )* prefix)
            }

            fn hint(&self, $( $arg_name: &$arg_type, )*) -> Vec<String> {
                self.parser.hint($( $arg_name, )*)
            }
        }
    }
}

define_arg_parser_map!(
    Arg2Map: Arg2Parser,
    { res1: Res1 (_res1), },
    Res2A, Res2B
);

// define_arg_parser_map!(
//     Arg3Map: Arg3Parser,
//     { res1: Res1 (_res1), res2: Res2 (_res2), },
//     Res3A, Res3B
// );

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::{Arg2Map, ContextFreeMap};

    use crate::input::arg_parser::prim_int_for_range;
    use crate::input::arg_parser::test_utils::{
        build_arg2_parse_checkers, build_cf_parse_checkers,
    };
    use crate::input::arg_parser::ContextFreeArgParser;

    #[test]
    fn simple_context_free_parser_adapter() {
        #[derive(PartialEq, Clone, Debug)]
        enum LeftOrRight {
            Left(u8),
            Right(u8),
        };

        use LeftOrRight::*;

        let parser = {
            let int_parser = prim_int_for_range(0, 99);

            ContextFreeMap::new(int_parser, |v| {
                if v < 50 {
                    Left(v)
                } else {
                    Right(v - 50)
                }
            })
        };

        let expected_hint = &["<0-99>"];
        let expected_above_hint = &["max: 99"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("parser", parser);

        check_hint(expected_hint);

        check_parse("0", Left(0));
        check_parse("17", Left(17));
        check_parse("49", Left(49));
        check_parse("50", Right(0));
        check_parse("51", Right(1));
        check_parse("99", Right(49));

        check_failure("-1", 2, expected_hint);
        check_failure("", 0, expected_hint);
        check_failure("a", 0, expected_hint);
        check_failure("test", 0, expected_hint);
        check_failure("*", 0, expected_hint);
        check_failure("100", 3, expected_above_hint);
        check_failure("255", 3, expected_above_hint);

        check_suggestions("", &[]);
        check_suggestions("1", &[]);
        check_suggestions("0", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn simple_arg_2_parser_adapter() {
        #[derive(PartialEq, Clone, Debug)]
        enum LeftOrRight {
            Left(u8, u8),
            Right(String, u8),
        };

        use LeftOrRight::*;

        let parser = {
            let int_parser = prim_int_for_range(0u8, 99);

            Arg2Map::new(int_parser.adapt(), |arg1, v| {
                if v < 50 {
                    Left(*arg1, v)
                } else {
                    Right(arg1.to_string(), v - 50)
                }
            })
        };

        let expected_hint = &["<0-99>"];
        let expected_above_hint = &["max: 99"];

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_arg2_parse_checkers("parser", parser);

        check_hint(&0, expected_hint);

        check_parse(&11, "0", Left(11, 0));
        check_parse(&7, "17", Left(7, 17));
        check_parse(&0, "49", Left(0, 49));
        check_parse(&8, "50", Right("8".to_string(), 0));
        check_parse(&34, "51", Right("34".to_string(), 1));
        check_parse(&255, "99", Right("255".to_string(), 49));

        check_failure(&0, "-1", 2, expected_hint);
        check_failure(&0, "", 0, expected_hint);
        check_failure(&10, "a", 0, expected_hint);
        check_failure(&4, "test", 0, expected_hint);
        check_failure(&0, "*", 0, expected_hint);
        check_failure(&100, "100", 3, expected_above_hint);
        check_failure(&255, "255", 3, expected_above_hint);

        check_suggestions(&0, "", &[]);
        check_suggestions(&3, "1", &[]);
        check_suggestions(&10, "0", &[]);
        check_suggestions(&7, "a", &[]);
    }
}
