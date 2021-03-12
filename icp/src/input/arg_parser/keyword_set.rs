use std::string::ToString;

use super::{ArgParseRes, ContextFreeArgParser};

#[derive(PartialEq, Clone, Debug)]
pub struct KeywordSetArgParser {
    keywords: Vec<String>,
    hints: Vec<String>,
}

impl KeywordSetArgParser {
    fn new(keywords: Vec<String>, hints: Vec<String>) -> Self {
        if keywords.is_empty() {
            panic!("`keywords` should not be empty");
        }

        Self { keywords, hints }
    }
}

pub fn keyword_set<Keyword, Keywords>(keywords: Keywords) -> KeywordSetArgParser
where
    Keyword: ToString,
    Keywords: IntoIterator<Item = Keyword>,
{
    let keywords = keywords
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let hints = keywords.clone();
    KeywordSetArgParser::new(keywords, hints)
}

pub fn keyword_set_with_hint<Keyword, Keywords, Hint, Hints>(
    keywords: Keywords,
    hints: Hints,
) -> KeywordSetArgParser
where
    Keyword: ToString,
    Keywords: IntoIterator<Item = Keyword>,
    Hint: ToString,
    Hints: IntoIterator<Item = Hint>,
{
    let keywords = keywords
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let hints = hints.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    KeywordSetArgParser::new(keywords, hints)
}

fn common_prefix_len(s1: &str, s2: &str) -> usize {
    let mut matched = 0;
    for (res_char, next_char) in s1.chars().zip(s2.chars()) {
        if res_char != next_char {
            break;
        }
        matched += 1;
    }
    matched
}

impl ContextFreeArgParser<String> for KeywordSetArgParser {
    fn parse(&self, input: &str) -> ArgParseRes<String> {
        for k in &self.keywords {
            if input == k {
                return ArgParseRes::Parsed(k.clone());
            }
        }

        let longest_match = self
            .keywords
            .iter()
            .map(|k| common_prefix_len(input, &k))
            .max()
            .unwrap_or(0);

        ArgParseRes::Failed {
            parsed_up_to: longest_match,
            reason: self.hints.clone(),
        }
    }

    fn suggestion(&self, prefix: &str) -> Vec<String> {
        self.keywords
            .iter()
            .filter(|k| k.starts_with(prefix) && k.len() > prefix.len())
            .cloned()
            .collect()
    }

    fn hint(&self) -> Vec<String> {
        self.hints.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{keyword_set, keyword_set_with_hint, ContextFreeArgParser};

    use crate::input::arg_parser::test_utils::build_cf_parse_checkers;

    #[test]
    fn simple_set() {
        let ks = &["full", "half", "halt", "hallo"];
        let expected_hint = ks;

        let parser = keyword_set(ks);

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("parser", parser);

        check_hint(expected_hint);

        check_parse("full", "full".to_string());
        check_parse("half", "half".to_string());
        check_parse("halt", "halt".to_string());
        check_parse("hallo", "hallo".to_string());

        check_failure("ful", 3, expected_hint);
        check_failure("fulll", 4, expected_hint);
        check_failure("abc", 0, expected_hint);
        check_failure("334", 0, expected_hint);
        check_failure("", 0, expected_hint);
        check_failure("h", 1, expected_hint);
        check_failure("hal", 3, expected_hint);

        check_suggestions("", &["full", "half", "halt", "hallo"]);
        check_suggestions("f", &["full"]);
        check_suggestions("fu", &["full"]);
        check_suggestions("ful", &["full"]);
        check_suggestions("full", &[]);
        check_suggestions("h", &["half", "halt", "hallo"]);
        check_suggestions("ha", &["half", "halt", "hallo"]);
        check_suggestions("hal", &["half", "halt", "hallo"]);
        check_suggestions("half", &[]);
        check_suggestions("halt", &[]);
        check_suggestions("hall", &["hallo"]);
        check_suggestions("hallo", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn wth_hint() {
        let ks = &["full", "half", "halt", "hallo"];
        let hints = &["several", "hints"];

        let parser = keyword_set_with_hint(ks, hints);

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("parser", parser);

        check_hint(hints);

        check_parse("full", "full".to_string());
        check_parse("half", "half".to_string());
        check_parse("halt", "halt".to_string());
        check_parse("hallo", "hallo".to_string());

        check_failure("ful", 3, hints);
        check_failure("fulll", 4, hints);
        check_failure("abc", 0, hints);
        check_failure("334", 0, hints);
        check_failure("", 0, hints);
        check_failure("h", 1, hints);
        check_failure("hal", 3, hints);

        check_suggestions("", &["full", "half", "halt", "hallo"]);
        check_suggestions("f", &["full"]);
        check_suggestions("fu", &["full"]);
        check_suggestions("ful", &["full"]);
        check_suggestions("full", &[]);
        check_suggestions("h", &["half", "halt", "hallo"]);
        check_suggestions("ha", &["half", "halt", "hallo"]);
        check_suggestions("hal", &["half", "halt", "hallo"]);
        check_suggestions("half", &[]);
        check_suggestions("halt", &[]);
        check_suggestions("hall", &["hallo"]);
        check_suggestions("hallo", &[]);
        check_suggestions("a", &[]);
    }

    #[test]
    fn map() {
        #[derive(PartialEq, Clone, Debug)]
        enum HalfOrFull {
            Half,
            Full,
        };

        use HalfOrFull::*;

        let ks = &["full", "half"];
        let expected_hint = ks;
        let keyword_arg = keyword_set(ks);

        let typed_arg = keyword_arg.map(|s| match s.as_str() {
            "half" => Half,
            "full" => Full,
            _ => panic!("Unexpected keyword"),
        });

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("typed_arg", typed_arg);

        check_hint(expected_hint);

        check_parse("full", Full);
        check_parse("half", Half);

        check_failure("ful", 3, expected_hint);
        check_failure("fulll", 4, expected_hint);
        check_failure("abc", 0, expected_hint);
        check_failure("334", 0, expected_hint);
        check_failure("", 0, expected_hint);
        check_failure("h", 1, expected_hint);
        check_failure("hal", 3, expected_hint);

        check_suggestions("", expected_hint);
        check_suggestions("h", &["half"]);
        check_suggestions("he", &[]);
        check_suggestions("f", &["full"]);
        check_suggestions("full", &[]);
        check_suggestions("fulle", &[]);
        check_suggestions("z", &[]);
    }
}
