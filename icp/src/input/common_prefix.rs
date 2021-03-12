//! A helper to find a common prefix of a set of strings.

use std::cmp::min;

pub fn common_prefix<'a>(
    mut options: impl Iterator<Item = &'a str>,
) -> &'a str {
    let mut res = match options.next() {
        None => "",
        Some(option) => option,
    };

    for next in options {
        let mut matched_end = min(res.len(), next.len());
        for ((res_char_i, res_char), next_char) in
            res.char_indices().zip(next.chars())
        {
            if res_char != next_char {
                matched_end = res_char_i;
                break;
            }
        }

        res = &res[0..matched_end];
    }

    res
}

#[cfg(test)]
mod tests {
    use super::common_prefix;

    #[test]
    fn common_prefix_basic() {
        assert_eq!(common_prefix(vec![].into_iter()), "".to_string());

        assert_eq!(common_prefix(vec!["abc", "def"].into_iter()), "");
        assert_eq!(common_prefix(vec!["abc", "axy"].into_iter()), "a");
        assert_eq!(common_prefix(vec!["abc", "axy", "def"].into_iter()), "");
        assert_eq!(common_prefix(vec!["abc", "aby", "abef"].into_iter()), "ab");
    }
}
