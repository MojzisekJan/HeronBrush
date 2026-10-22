//! String formatting and trimming helpers.
//!
//! Case conversion is ASCII-only.

/// The characters treated as whitespace by default.
pub const WHITESPACE: &str = " \n\t\r";

/// The character used to escape another by default.
pub const ESCAPE_CHAR: char = '\\';

/// Returns `singular` if `count` is 1, and `plural` otherwise.
#[inline]
pub fn str_plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

/// Returns `prefix`, then the appropriate one of `singular` and `plural`, then `suffix`.
pub fn str_plural_wrapped(
    prefix: &str,
    count: usize,
    singular: &str,
    plural: &str,
    suffix: &str,
) -> String {
    format!("{prefix}{}{suffix}", str_plural(count, singular, plural))
}

/// Returns the given string with the given characters stripped from both ends.
pub fn str_trim<'a>(s: &'a str, chars: &str) -> &'a str {
    s.trim_matches(|c| chars.contains(c))
}

/// Returns the given string with whitespace stripped from both ends.
#[inline]
pub fn str_trim_whitespace(s: &str) -> &str {
    str_trim(s, WHITESPACE)
}

/// Returns the given string in lower case.
#[inline]
pub fn str_to_lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// Returns the given string in upper case.
#[inline]
pub fn str_to_upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// Capitalizes the first character of each word.
///
/// A word starts at the beginning of the string and after any character in `delims`.
pub fn str_capitalize(s: &str, delims: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut at_word_start = true;

    for c in s.chars() {
        if delims.contains(c) {
            at_word_start = true;
            result.push(c);
        } else if at_word_start {
            result.push(c.to_ascii_uppercase());
            at_word_start = false;
        } else {
            result.push(c);
            at_word_start = false;
        }
    }
    result
}

/// Escapes every character of `chars`, and the escape character itself, by prefixing it
/// with `esc`.
pub fn str_escape(s: &str, chars: &str, esc: char) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        if c == esc || chars.contains(c) {
            result.push(esc);
        }
        result.push(c);
    }
    result
}

/// Escapes as [`str_escape`] does, but leaves already-escaped characters alone.
///
/// A backslash that is already escaping one of `chars` is left as it is, so that applying
/// this to a partly escaped string does not double up its escapes.
pub fn str_escape_if_necessary(s: &str, chars: &str, esc: char) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars_iter = s.chars().peekable();

    while let Some(c) = chars_iter.next() {
        if c == esc {
            match chars_iter.peek() {
                // Already a valid escape sequence; pass both characters through.
                Some(&next) if next == esc || chars.contains(next) => {
                    result.push(c);
                    result.push(next);
                    chars_iter.next();
                }
                _ => {
                    result.push(esc);
                    result.push(c);
                }
            }
        } else {
            if chars.contains(c) {
                result.push(esc);
            }
            result.push(c);
        }
    }
    result
}

/// Undoes [`str_escape`].
///
/// An escape character in front of anything not in `chars` is kept along with what
/// follows it, so an unrecognized escape sequence survives a round trip rather than
/// losing its backslash.
pub fn str_unescape(s: &str, chars: &str, esc: char) -> String {
    let mut result = String::with_capacity(s.len());
    let mut escaped = false;

    for c in s.chars() {
        if c == esc {
            if escaped {
                result.push(c);
            }
            escaped = !escaped;
        } else {
            if escaped && !chars.contains(c) {
                result.push(esc);
            }
            result.push(c);
            escaped = false;
        }
    }

    // A trailing escape character escapes nothing, so it stands for itself.
    if escaped {
        result.push(esc);
    }
    result
}

/// Returns whether the given string is empty or contains only characters from
/// `whitespace`.
pub fn str_is_blank(s: &str, whitespace: &str) -> bool {
    s.chars().all(|c| whitespace.contains(c))
}

/// Returns whether the given string consists only of decimal digits.
///
/// An empty string is accepted as numeric.
pub fn str_is_numeric(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural() {
        assert_eq!(str_plural(1, "brush", "brushes"), "brush");
        assert_eq!(str_plural(0, "brush", "brushes"), "brushes");
        assert_eq!(str_plural(2, "brush", "brushes"), "brushes");
        assert_eq!(
            str_plural_wrapped("Delete ", 2, "brush", "brushes", "?"),
            "Delete brushes?"
        );
    }

    #[test]
    fn trim() {
        assert_eq!(str_trim_whitespace("  hello \t\n"), "hello");
        assert_eq!(str_trim_whitespace(""), "");
        assert_eq!(str_trim_whitespace("   "), "");
        assert_eq!(str_trim("xxhelloxx", "x"), "hello");
    }

    #[test]
    fn case_conversion() {
        assert_eq!(str_to_lower("Hello World"), "hello world");
        assert_eq!(str_to_upper("Hello World"), "HELLO WORLD");
    }

    #[test]
    fn capitalize() {
        assert_eq!(str_capitalize("hello world", WHITESPACE), "Hello World");
        assert_eq!(str_capitalize("", WHITESPACE), "");
        assert_eq!(str_capitalize("a", WHITESPACE), "A");
        // Delimiters are kept, and repeated ones do not upset the word tracking.
        assert_eq!(str_capitalize("hello  world", WHITESPACE), "Hello  World");
    }

    #[test]
    fn escape_and_unescape_round_trip() {
        let escaped = str_escape("say \"hi\"", "\"", ESCAPE_CHAR);
        assert_eq!(escaped, "say \\\"hi\\\"");
        assert_eq!(str_unescape(&escaped, "\"", ESCAPE_CHAR), "say \"hi\"");

        // The escape character itself is escaped.
        let escaped = str_escape("a\\b", "", ESCAPE_CHAR);
        assert_eq!(escaped, "a\\\\b");
        assert_eq!(str_unescape(&escaped, "", ESCAPE_CHAR), "a\\b");

        assert_eq!(str_escape("", "\"", ESCAPE_CHAR), "");
        assert_eq!(str_unescape("", "\"", ESCAPE_CHAR), "");
    }

    #[test]
    fn escape_if_necessary_leaves_existing_escapes_alone() {
        assert_eq!(
            str_escape_if_necessary("a\\\"b", "\"", ESCAPE_CHAR),
            "a\\\"b"
        );
        assert_eq!(str_escape_if_necessary("a\"b", "\"", ESCAPE_CHAR), "a\\\"b");
    }

    #[test]
    fn unescape_keeps_an_unrecognized_escape() {
        // `\n` is not in the escapable set, so the backslash is not consumed.
        assert_eq!(str_unescape("a\\nb", "\"", ESCAPE_CHAR), "a\\nb");
        // A trailing backslash stands for itself.
        assert_eq!(str_unescape("ab\\", "\"", ESCAPE_CHAR), "ab\\");
    }

    #[test]
    fn is_blank() {
        assert!(str_is_blank("", WHITESPACE));
        assert!(str_is_blank("  \t\n", WHITESPACE));
        assert!(!str_is_blank(" a ", WHITESPACE));
    }

    #[test]
    fn is_numeric() {
        assert!(str_is_numeric("12345"));
        assert!(str_is_numeric(""));
        assert!(!str_is_numeric("12a45"));
        assert!(!str_is_numeric("-1"));
    }
}
