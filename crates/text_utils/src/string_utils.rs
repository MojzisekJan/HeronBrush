//! String splitting, tokenizing and searching.
//!
//! Only the parts with behaviour of their own are here. `str_join` is
//! [`slice::join`](https://doc.rust-lang.org/std/primitive.slice.html#method.join), and
//! the `str_to_int` / `str_to_double` family is [`str::parse`], so neither is reproduced.

/// A delimited substring found by [`str_find_next_delimited_string`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelimitedString {
    /// The byte offset of the opening delimiter.
    pub start: usize,
    /// The length in bytes from the opening delimiter through the closing one, or `None`
    /// if the string was never closed.
    pub length: Option<usize>,
}

/// Splits the given string on any of the given delimiter characters, dropping empty
/// tokens.
///
/// Unlike [`str::split`], skips empty tokens and accepts multiple delimiter characters.
pub fn str_split(s: &str, delims: &str) -> Vec<String> {
    s.split(|c| delims.contains(c))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

/// Returns the byte range of the next token in `s`, skipping any leading delimiters.
///
/// Returns `None` when only delimiters remain.
pub fn str_next_token(s: &str, delims: &str) -> Option<(usize, usize)> {
    let start = s.find(|c| !delims.contains(c))?;
    let end = s[start..]
        .find(|c| delims.contains(c))
        .map_or(s.len(), |offset| start + offset);
    Some((start, end))
}

/// Replaces every occurrence of `needle` in `haystack` with `replacement`.
///
/// Returns the haystack unchanged when either operand is empty or both are equal, preventing
/// non-progressing replacement.
pub fn str_replace_every(haystack: &str, needle: &str, replacement: &str) -> String {
    if haystack.is_empty() || needle.is_empty() || needle == replacement {
        return haystack.to_string();
    }
    haystack.replace(needle, replacement)
}

/// Finds the next substring of `s` delimited by `start_delim` and `end_delim`.
///
/// Nested pairs are skipped, so the result spans the outermost pair. A delimiter preceded
/// by `escape_char` is treated as an ordinary character. If an opening delimiter is found
/// but never closed, the result's [`DelimitedString::length`] is `None`.
pub fn str_find_next_delimited_string(
    s: &str,
    start_delim: &str,
    end_delim: &str,
    escape_char: Option<char>,
) -> Option<DelimitedString> {
    let mut start: Option<usize> = None;
    let mut depth = 0usize;
    let mut escaped = false;

    for (i, c) in s.char_indices() {
        if Some(c) == escape_char && !escaped {
            escaped = true;
            continue;
        }

        if !escaped && s[i..].starts_with(start_delim) {
            match start {
                None => start = Some(i),
                Some(_) => depth += 1,
            }
        } else if !escaped && start.is_some() && s[i..].starts_with(end_delim) {
            if depth == 0 {
                let start = start.expect("checked just above");
                return Some(DelimitedString {
                    start,
                    length: Some(i + end_delim.len() - start),
                });
            }
            depth -= 1;
        }
        escaped = false;
    }

    // An opening delimiter with no match: report where it was, but not a length.
    start.map(|start| DelimitedString {
        start,
        length: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_drops_empty_tokens() {
        assert_eq!(str_split("a b c", " "), vec!["a", "b", "c"]);
        // Runs of delimiters do not produce empty tokens.
        assert_eq!(str_split("a  b", " "), vec!["a", "b"]);
        assert_eq!(str_split("  a  ", " "), vec!["a"]);
        assert_eq!(str_split("", " "), Vec::<String>::new());
        assert_eq!(str_split("   ", " "), Vec::<String>::new());
        // Any of the given characters is a delimiter.
        assert_eq!(str_split("a,b;c", ",;"), vec!["a", "b", "c"]);
    }

    #[test]
    fn next_token() {
        assert_eq!(str_next_token("  ab cd", " "), Some((2, 4)));
        assert_eq!(str_next_token("ab", " "), Some((0, 2)));
        assert_eq!(str_next_token("   ", " "), None);
        assert_eq!(str_next_token("", " "), None);
    }

    #[test]
    fn replace_every() {
        assert_eq!(str_replace_every("aXbXc", "X", "-"), "a-b-c");
        assert_eq!(str_replace_every("aXbXc", "X", ""), "abc");
        assert_eq!(str_replace_every("abc", "X", "-"), "abc");
        // Empty and identical operands must terminate without repeated replacement.
        assert_eq!(str_replace_every("", "X", "-"), "");
        assert_eq!(str_replace_every("abc", "", "-"), "abc");
        assert_eq!(str_replace_every("abc", "b", "b"), "abc");
    }

    #[test]
    fn find_next_delimited_string() {
        let found = str_find_next_delimited_string("a {b} c", "{", "}", None).unwrap();
        assert_eq!(found.start, 2);
        assert_eq!(found.length, Some(3));

        assert_eq!(str_find_next_delimited_string("abc", "{", "}", None), None);
    }

    #[test]
    fn find_next_delimited_string_spans_nested_pairs() {
        let found = str_find_next_delimited_string("{a {b} c}", "{", "}", None).unwrap();
        assert_eq!(found.start, 0);
        // The whole outermost pair, not the inner one.
        assert_eq!(found.length, Some(9));
    }

    #[test]
    fn find_next_delimited_string_reports_an_unterminated_string() {
        let found = str_find_next_delimited_string("a {b c", "{", "}", None).unwrap();
        assert_eq!(found.start, 2);
        assert_eq!(found.length, None);
    }

    #[test]
    fn find_next_delimited_string_honours_escapes() {
        // The escaped brace does not open anything, so the later one does.
        let found = str_find_next_delimited_string("\\{a {b} c", "{", "}", Some('\\')).unwrap();
        assert_eq!(found.start, 4);
        assert_eq!(found.length, Some(3));

        // An escaped closing brace does not close the string.
        let found = str_find_next_delimited_string("{a \\} b}", "{", "}", Some('\\')).unwrap();
        assert_eq!(found.length, Some(8));
    }
}
