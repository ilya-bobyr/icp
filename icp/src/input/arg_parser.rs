//! A "quick and dirty" parser for command arguments, see
//! [`input::command_parser`] for details.
//!
//! Argument parsers are instances of the [`ContextFreeArgParser`], and
//! [`Arg2Parser`] traits.  Traits for additional arguments can be generated by
//! the [`define_arg_parser`] macro, if necessary.
//!
//! There are predefined parsers for argument types that are commonly used in
//! PET.  Se the child pacakges of the [`input::arg_parser`] package.
//!
//! See documentation for individual traits and methods for additional details.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use map::{Arg2Map, ContextFreeMap};

pub mod alternatives;
pub mod file;
pub mod keyword_set;
pub mod map;
pub mod prim_int;

pub mod test_utils;

#[cfg(test)]
pub use alternatives::alternatives_arg2;
pub use alternatives::alternatives_cf;
#[cfg(test)]
pub use file::file;
pub use file::file_for_current_dir;
pub use keyword_set::{keyword_set, keyword_set_with_hint};
#[cfg(test)]
pub use prim_int::{prim_int, prim_int_for_range};
pub use prim_int::{prim_int_for_range_and_name, prim_int_with_name};

/// Result of parsing an argument.  Value returned by the
/// [`ContextFreeArgParser::parse()`] and [`Arg2Parser::parse()`] methods.
#[derive(PartialEq, Clone, Debug)]
pub enum ArgParseRes<Res> {
    Failed {
        /// Last input character that was successfully parsed.  `0` means that
        /// parsing of this argument have failed at the "syntactic level".  If
        /// the argument had the right structure but failed at a higher level
        /// (like bounds checks), this should point to the last character of the
        /// argument, not the first.
        parsed_up_to: usize,

        /// When additional information is available as to why the parsing
        /// failed, this field holds the explanation.  There could be more than
        /// one failure.  For example if several parsers where applied and all
        /// have failed.
        ///
        /// Note that all listed failures apply only to the parsed part.  If
        /// several parsers where applied and one failed at a later point that
        /// failure should not be present in this list.  So all the values
        /// should be shown to the users.
        reason: Vec<String>,
    },
    Parsed(Res),
}

impl<Res> ArgParseRes<Res> {
    /// Combines two `ArgParseRes` results, giving `self` preference in case
    /// they are considered equal.
    ///
    /// `Parsed` is preferred over any kind of failure.  One failure is
    /// preferred over another if the first happened at a later position.
    ///
    /// When multiple failures happen at the same position, their reasons are
    /// combined.
    pub fn merge(self, other: ArgParseRes<Res>) -> ArgParseRes<Res> {
        match (self, other) {
            (me @ ArgParseRes::Parsed(_), _other) => me,
            (_me, other @ ArgParseRes::Parsed(_)) => other,
            (
                ArgParseRes::Failed {
                    parsed_up_to: me_parsed_up_to,
                    reason: mut me_reason,
                },
                ArgParseRes::Failed {
                    parsed_up_to: other_parsed_up_to,
                    reason: mut other_reason,
                },
            ) => match me_parsed_up_to.cmp(&other_parsed_up_to) {
                Ordering::Less => ArgParseRes::Failed {
                    parsed_up_to: other_parsed_up_to,
                    reason: other_reason,
                },
                Ordering::Greater => ArgParseRes::Failed {
                    parsed_up_to: me_parsed_up_to,
                    reason: me_reason,
                },
                Ordering::Equal => {
                    me_reason.append(&mut other_reason);
                    ArgParseRes::Failed {
                        parsed_up_to: me_parsed_up_to,
                        reason: me_reason,
                    }
                }
            },
        }
    }
}

/// A context-free argument parser - it only sees its own argument and produces
/// a result based on that.
pub trait ContextFreeArgParser<Res> {
    /// Parse the input as a command argument and either return a parsed
    /// representation or an explanation of what was expected.
    fn parse(&self, input: &str) -> ArgParseRes<Res>;

    /// Try to parse an argument prefix and produce a list of possible ways to
    /// complete the argument to form something that [`parse`] will parse
    /// correctly.
    fn suggestion(&self, prefix: &str) -> Vec<String>;

    /// Hint as to what this argument is expected to look like.  In case an
    /// argument may have several forms, they should be returned as separate
    /// elements of the vector.
    fn hint(&self) -> Vec<String>;

    /// Creates a new parser that maps the result of the current parser using a
    /// function.
    fn map<F, B>(self, f: F) -> ContextFreeMap<Res, B, Self, F>
    where
        F: Fn(Res) -> B,
        Self: Sized,
    {
        ContextFreeMap::new(self, f)
    }

    /// Allows a context free parser to be used as a non-context free parser, as
    /// `ContextFreeAdapter` implements `Arg2Parser` and friends.
    fn adapt(self) -> ContextFreeAdapter<Self, Res>
    where
        Self: Sized,
    {
        ContextFreeAdapter::new(self)
    }

    /// It is not uncommon to box parsers, in particular when we want to put
    /// parsers of different types into a vector.  This method helps to remove
    /// some of the syntactic noise.
    fn boxed(self) -> Box<dyn ContextFreeArgParser<Res>>
    where
        Self: Sized + 'static,
    {
        Box::new(self) as Box<dyn ContextFreeArgParser<Res>>
    }
}

/// A helper wrapper that implements all the `Arg2Parser` and friends, while
/// forwarding to a contained `ContextFreeArgParser`.  Use
/// [`ContextFreeArgParser::adapt()`], instead of using this type directly.
pub struct ContextFreeAdapter<Parser, Res>
where
    Parser: ContextFreeArgParser<Res>,
{
    parser: Parser,
    _res: PhantomData<Res>,
}

impl<Parser, Res> ContextFreeAdapter<Parser, Res>
where
    Parser: ContextFreeArgParser<Res>,
{
    pub fn new(parser: Parser) -> Self {
        Self {
            parser,
            _res: PhantomData,
        }
    }
}

impl<Parser, Res> ContextFreeArgParser<Res> for ContextFreeAdapter<Parser, Res>
where
    Parser: ContextFreeArgParser<Res>,
{
    fn parse(&self, input: &str) -> ArgParseRes<Res> {
        self.parser.parse(input)
    }
    fn suggestion(&self, prefix: &str) -> Vec<String> {
        self.parser.suggestion(prefix)
    }
    fn hint(&self) -> Vec<String> {
        self.parser.hint()
    }
}

impl<T, Res> ContextFreeArgParser<Res> for Rc<T>
where
    T: ContextFreeArgParser<Res>,
{
    fn parse(&self, input: &str) -> ArgParseRes<Res> {
        self.as_ref().parse(input)
    }
    fn suggestion(&self, prefix: &str) -> Vec<String> {
        self.as_ref().suggestion(prefix)
    }
    fn hint(&self) -> Vec<String> {
        self.as_ref().hint()
    }
}

impl<T, Res> ContextFreeArgParser<Res> for RefCell<T>
where
    T: ContextFreeArgParser<Res>,
{
    fn parse(&self, input: &str) -> ArgParseRes<Res> {
        self.borrow().parse(input)
    }
    fn suggestion(&self, prefix: &str) -> Vec<String> {
        self.borrow().suggestion(prefix)
    }
    fn hint(&self) -> Vec<String> {
        self.borrow().hint()
    }
}

/// Generates "context-sensitive" argument parser traits - ones that consider
/// values of all the preceding arguments when parsing the current argument.
///
/// All context-free argument parsers and automatically context-sensitive, when
/// the just ignore values produced by preceding parsers, and the macro will
/// generate corresponding instances.
///
/// Except for additional arguments holding references to the parse context,
/// generated traits are identical to the [`ContextFreeArgParser`] trait.
macro_rules! define_arg_parser {
    ($name:ident,
     { $( $arg_name:ident : $arg_type:ident ),* $(,)* },
     $res:ident,
     $map_name:ident, $mapped_res:ident
     $(,)*
    ) => {
        pub trait $name<$( $arg_type, )* $res> {
            fn parse(&self, $( $arg_name: &$arg_type, )* input: &str)
                -> ArgParseRes<$res>;
            fn suggestion(&self, $( $arg_name: &$arg_type, )* prefix: &str)
                -> Vec<String>;
            fn hint(&self, $( $arg_name: &$arg_type, )*) -> Vec<String>;

            /// Creates a new parser that maps the result of the current parser
            /// using a function.
            fn map<F, $mapped_res>(self, f: F)
                -> $map_name<$( $arg_type, )* $res, $mapped_res, Self, F>
            where
                F: Fn($( &$arg_type, )* $res) -> $mapped_res,
                Self: Sized,
            {
                $map_name::new(self, f)
            }

            /// It is not uncommon to box parsers, in particular when we want to
            /// put parsers of different types into a vector.  This method helps
            /// to remove some of the syntactic noise.
            fn boxed(self) -> Box<dyn $name<$( $arg_type, )* $res>>
            where
                Self: Sized + 'static,
            {
                Box::new(self) as Box<dyn $name<$( $arg_type, )* $res>>
            }
        }

        impl<Parser, $( $arg_type, )* $res> $name<$( $arg_type, )* $res>
            for ContextFreeAdapter<Parser, $res>
        where
            Parser: ContextFreeArgParser<$res>,
        {
            fn parse(&self, $( _: &$arg_type, )* input: &str)
                -> ArgParseRes<$res>
            {
                self.parser.parse(input)
            }

            fn suggestion(&self, $( _: &$arg_type, )* prefix: &str)
                -> Vec<String>
            {
                self.parser.suggestion(prefix)
            }

            fn hint(&self, $( _: &$arg_type, )*) -> Vec<String> {
                self.parser.hint()
            }
        }

        impl<T, $( $arg_type, )* $res> $name<$( $arg_type, )* $res> for Box<T>
        where
            T: ContextFreeArgParser<$res>
        {
            fn parse(&self, $( _: &$arg_type, )* input: &str)
                -> ArgParseRes<$res>
            {
                <T as ContextFreeArgParser<$res>>::parse(self.deref(), input)
            }

            fn suggestion(&self, $( _: &$arg_type, )* prefix: &str)
                -> Vec<String>
            {
                <T as ContextFreeArgParser<$res>>
                    ::suggestion(self.deref(), prefix)
            }

            fn hint(&self, $( _: &$arg_type, )*) -> Vec<String> {
                <T as ContextFreeArgParser<$res>>::hint(self.deref())
            }
        }

        impl<T, $( $arg_type, )* $res> $name<$( $arg_type, )* $res> for Rc<T>
        where
            T: $name<$( $arg_type, )* $res>
        {
            fn parse(&self, $( $arg_name: &$arg_type, )* input: &str)
                -> ArgParseRes<$res>
            {
                self.as_ref().parse($( $arg_name, )* input)
            }

            fn suggestion(&self, $( $arg_name: &$arg_type, )* prefix: &str)
                -> Vec<String>
            {
                self.as_ref().suggestion($( $arg_name, )* prefix)
            }

            fn hint(&self, $( $arg_name: &$arg_type, )*) -> Vec<String> {
                self.as_ref().hint($( $arg_name, )*)
            }
        }

        impl<T, $( $arg_type, )* $res> $name<$( $arg_type, )* $res> for
            RefCell<T>
        where
            T: $name<$( $arg_type, )* $res>
        {
            fn parse(&self, $( $arg_name: &$arg_type, )* input: &str)
                -> ArgParseRes<$res>
            {
                self.borrow().parse($( $arg_name, )* input)
            }

            fn suggestion(&self, $( $arg_name: &$arg_type, )* prefix: &str)
                -> Vec<String>
            {
                self.borrow().suggestion($( $arg_name, )* prefix)
            }

            fn hint(&self, $( $arg_name: &$arg_type, )*) -> Vec<String> {
                self.borrow().hint($( $arg_name, )*)
            }
        }
    };
}

// Arg1Parser is ContextFreeArgParser.  Context is the values of the previous
// arguments, and thus the first argument has not context.
define_arg_parser!(
    Arg2Parser,
    { res1: Res1, },
    Res2,
    Arg2Map, Res2B,
);

// define_arg_parser!(
//     Arg3Parser,
//     { res1: Res1, res2: Res2, },
//     Res3,
//     Arg3Map, Res3B,
// );
