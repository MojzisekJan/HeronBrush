//! String comparison and glob matching, in case-sensitive and case-insensitive variants.
//!
//! Shared comparison functions take [`CaseSensitivity`]; [`cs`] and [`ci`] provide case-sensitive
//! and case-insensitive entry points.
//!
//! Case folding is ASCII-only for material, entity and property names.
//!
//! Glob matching operates on Unicode characters, so `?` matches one character even when its UTF-8
//! encoding spans multiple bytes.

use std::cmp::Ordering;

/// Whether a comparison distinguishes upper from lower case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaseSensitivity {
    /// `'a'` and `'A'` are different characters.
    Sensitive,
    /// `'a'` and `'A'` are the same character.
    Insensitive,
}

impl CaseSensitivity {
    /// Folds a character for comparison under this sensitivity.
    #[inline]
    fn fold(self, c: char) -> char {
        match self {
            CaseSensitivity::Sensitive => c,
            CaseSensitivity::Insensitive => c.to_ascii_lowercase(),
        }
    }

    /// Returns whether the two characters compare equal under this sensitivity.
    #[inline]
    pub fn char_equal(self, a: char, b: char) -> bool {
        self.fold(a) == self.fold(b)
    }

    /// Orders the two characters under this sensitivity.
    #[inline]
    pub fn char_compare(self, a: char, b: char) -> Ordering {
        self.fold(a).cmp(&self.fold(b))
    }
}

/// Returns the number of leading characters the two strings have in common.
pub fn str_mismatch(s1: &str, s2: &str, case: CaseSensitivity) -> usize {
    s1.chars()
        .zip(s2.chars())
        .take_while(|(a, b)| case.char_equal(*a, *b))
        .count()
}

/// Returns whether `haystack` contains `needle`.
///
/// An empty haystack contains no match, including an empty needle. A nonempty haystack contains an
/// empty needle, allowing empty filter patterns.
pub fn str_contains(haystack: &str, needle: &str, case: CaseSensitivity) -> bool {
    if haystack.is_empty() {
        return false;
    }
    if needle.is_empty() {
        return true;
    }

    match case {
        CaseSensitivity::Sensitive => haystack.contains(needle),
        CaseSensitivity::Insensitive => {
            let haystack: Vec<char> = haystack.chars().map(|c| c.to_ascii_lowercase()).collect();
            let needle: Vec<char> = needle.chars().map(|c| c.to_ascii_lowercase()).collect();
            needle.len() <= haystack.len() && haystack.windows(needle.len()).any(|w| w == needle)
        }
    }
}

/// Returns whether `haystack` starts with `needle`.
pub fn str_is_prefix(haystack: &str, needle: &str, case: CaseSensitivity) -> bool {
    match case {
        CaseSensitivity::Sensitive => haystack.starts_with(needle),
        CaseSensitivity::Insensitive => {
            let mut h = haystack.chars();
            needle
                .chars()
                .all(|n| h.next().is_some_and(|c| case.char_equal(c, n)))
        }
    }
}

/// Returns whether `haystack` ends with `needle`.
pub fn str_is_suffix(haystack: &str, needle: &str, case: CaseSensitivity) -> bool {
    match case {
        CaseSensitivity::Sensitive => haystack.ends_with(needle),
        CaseSensitivity::Insensitive => {
            let mut h = haystack.chars().rev();
            needle
                .chars()
                .rev()
                .all(|n| h.next().is_some_and(|c| case.char_equal(c, n)))
        }
    }
}

/// Lexicographically compares the two strings.
pub fn str_compare(s1: &str, s2: &str, case: CaseSensitivity) -> Ordering {
    let mut a = s1.chars();
    let mut b = s2.chars();
    loop {
        match (a.next(), b.next()) {
            (Some(x), Some(y)) => match case.char_compare(x, y) {
                Ordering::Equal => {}
                other => return other,
            },
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (None, None) => return Ordering::Equal,
        }
    }
}

/// Returns whether the two strings are equal.
#[inline]
pub fn str_is_equal(s1: &str, s2: &str, case: CaseSensitivity) -> bool {
    str_compare(s1, s2, case) == Ordering::Equal
}

/// Returns whether `s` matches the glob `pattern`.
///
/// The pattern language is the one used for material and entity name matching:
///
/// | pattern | matches |
/// |---|---|
/// | `*` | any sequence of characters, including none |
/// | `?` | exactly one character |
/// | `%` | exactly one digit |
/// | `%*` | any sequence of digits, including none |
/// | `\c` | the literal character `c`, where `c` is one of `*`, `?`, `%` or `\` |
///
/// Any other escape sequence fails the match.
pub fn str_matches_glob(s: &str, pattern: &str, case: CaseSensitivity) -> bool {
    let str_chars: Vec<char> = s.chars().collect();
    let pat_chars: Vec<char> = pattern.chars().collect();

    // A worklist of (string index, pattern index) positions still to try. Using an
    // explicit stack rather than recursion keeps a pattern full of `*` from blowing the
    // call stack.
    let mut tasks = vec![(0usize, 0usize)];

    while let Some((s_i, p_i)) = tasks.pop() {
        if s_i == str_chars.len() && p_i == pat_chars.len() {
            return true;
        }
        if p_i == pat_chars.len() {
            // The pattern ran out but the string did not.
            continue;
        }

        match pat_chars[p_i] {
            '\\' if p_i < pat_chars.len() - 1 => {
                if s_i < str_chars.len() {
                    let escaped = pat_chars[p_i + 1];
                    if matches!(escaped, '*' | '?' | '%' | '\\') {
                        if str_chars[s_i] == escaped {
                            tasks.push((s_i + 1, p_i + 2));
                        }
                    } else {
                        // An escape sequence that escapes nothing is an error, not a
                        // literal backslash.
                        return false;
                    }
                }
            }
            '*' => {
                if p_i == pat_chars.len() - 1 {
                    // A trailing `*` absorbs whatever is left.
                    return true;
                }
                if s_i == str_chars.len() {
                    tasks.push((s_i, p_i + 1));
                } else {
                    // Either the `*` stops here, or it swallows one more character.
                    tasks.push((s_i, p_i + 1));
                    tasks.push((s_i + 1, p_i));
                }
            }
            '?' => {
                if s_i < str_chars.len() {
                    tasks.push((s_i + 1, p_i + 1));
                }
            }
            '%' => {
                if p_i < pat_chars.len() - 1 && pat_chars[p_i + 1] == '*' {
                    // `%*`: any run of digits, including none.
                    tasks.push((s_i, p_i + 2));
                    if s_i < str_chars.len() && str_chars[s_i].is_ascii_digit() {
                        tasks.push((s_i + 1, p_i));
                    }
                } else if s_i < str_chars.len() && str_chars[s_i].is_ascii_digit() {
                    tasks.push((s_i + 1, p_i + 1));
                }
            }
            c => {
                if s_i < str_chars.len() && case.char_equal(c, str_chars[s_i]) {
                    tasks.push((s_i + 1, p_i + 1));
                }
            }
        }
    }

    false
}

/// Case-sensitive comparisons.
pub mod cs {
    use super::{CaseSensitivity, Ordering};

    /// The number of leading characters the two strings have in common.
    pub fn str_mismatch(s1: &str, s2: &str) -> usize {
        super::str_mismatch(s1, s2, CaseSensitivity::Sensitive)
    }
    /// Whether `haystack` contains `needle`.
    pub fn str_contains(haystack: &str, needle: &str) -> bool {
        super::str_contains(haystack, needle, CaseSensitivity::Sensitive)
    }
    /// Whether `haystack` starts with `needle`.
    pub fn str_is_prefix(haystack: &str, needle: &str) -> bool {
        super::str_is_prefix(haystack, needle, CaseSensitivity::Sensitive)
    }
    /// Whether `haystack` ends with `needle`.
    pub fn str_is_suffix(haystack: &str, needle: &str) -> bool {
        super::str_is_suffix(haystack, needle, CaseSensitivity::Sensitive)
    }
    /// Lexicographically compares the two strings.
    pub fn str_compare(s1: &str, s2: &str) -> Ordering {
        super::str_compare(s1, s2, CaseSensitivity::Sensitive)
    }
    /// Whether the two strings are equal.
    pub fn str_is_equal(s1: &str, s2: &str) -> bool {
        super::str_is_equal(s1, s2, CaseSensitivity::Sensitive)
    }
    /// Whether `s` matches the glob `pattern`.
    pub fn str_matches_glob(s: &str, pattern: &str) -> bool {
        super::str_matches_glob(s, pattern, CaseSensitivity::Sensitive)
    }
}

/// Case-insensitive comparisons.
pub mod ci {
    use super::{CaseSensitivity, Ordering};

    /// The number of leading characters the two strings have in common.
    pub fn str_mismatch(s1: &str, s2: &str) -> usize {
        super::str_mismatch(s1, s2, CaseSensitivity::Insensitive)
    }
    /// Whether `haystack` contains `needle`.
    pub fn str_contains(haystack: &str, needle: &str) -> bool {
        super::str_contains(haystack, needle, CaseSensitivity::Insensitive)
    }
    /// Whether `haystack` starts with `needle`.
    pub fn str_is_prefix(haystack: &str, needle: &str) -> bool {
        super::str_is_prefix(haystack, needle, CaseSensitivity::Insensitive)
    }
    /// Whether `haystack` ends with `needle`.
    pub fn str_is_suffix(haystack: &str, needle: &str) -> bool {
        super::str_is_suffix(haystack, needle, CaseSensitivity::Insensitive)
    }
    /// Lexicographically compares the two strings.
    pub fn str_compare(s1: &str, s2: &str) -> Ordering {
        super::str_compare(s1, s2, CaseSensitivity::Insensitive)
    }
    /// Whether the two strings are equal.
    pub fn str_is_equal(s1: &str, s2: &str) -> bool {
        super::str_is_equal(s1, s2, CaseSensitivity::Insensitive)
    }
    /// Whether `s` matches the glob `pattern`.
    pub fn str_matches_glob(s: &str, pattern: &str) -> bool {
        super::str_matches_glob(s, pattern, CaseSensitivity::Insensitive)
    }
}
