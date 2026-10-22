//! Tests for string compare behavior.

use std::cmp::Ordering;

use text_utils::string_compare::{ci, cs};

#[test]
fn str_mismatch() {
    assert_eq!(cs::str_mismatch("", ""), 0);
    assert_eq!(cs::str_mismatch("asdf", "asdf"), 4);
    assert_eq!(cs::str_mismatch("ssdf", "asdf"), 0);
    assert_eq!(cs::str_mismatch("aadf", "asdf"), 1);
    assert_eq!(cs::str_mismatch("asaf", "asdf"), 2);
    assert_eq!(cs::str_mismatch("asda", "asdf"), 3);
    // Case matters here but not there.
    assert_eq!(cs::str_mismatch("asdf", "Asdf"), 0);
    assert_eq!(cs::str_mismatch("asDf", "asdf"), 2);
    assert_eq!(ci::str_mismatch("asdf", "Asdf"), 4);
    assert_eq!(ci::str_mismatch("asDf", "asdf"), 4);
}

#[test]
fn str_contains() {
    // An empty haystack contains nothing, not even an empty needle.
    assert!(!cs::str_contains("", ""));
    assert!(cs::str_contains("asdf", ""));

    for needle in ["a", "s", "d", "f", "as", "sd", "df", "asd", "asdf"] {
        assert!(cs::str_contains("asdf", needle), "should contain {needle}");
    }
    for needle in ["m", "sdf2", "asf", "sde", "esd"] {
        assert!(
            !cs::str_contains("asdf", needle),
            "should not contain {needle}"
        );
    }

    for needle in ["Asdf", "A", "S", "D", "F", "ASDF"] {
        assert!(!cs::str_contains("asdf", needle));
        assert!(ci::str_contains("asdf", needle));
    }
}

#[test]
fn str_is_prefix() {
    for needle in ["", "a", "as", "asd", "asdf"] {
        assert!(cs::str_is_prefix("asdf", needle));
    }
    for needle in ["sdf", "aasdf", "df"] {
        assert!(!cs::str_is_prefix("asdf", needle));
    }

    for needle in ["A", "aS", "Asd", "asDF"] {
        assert!(!cs::str_is_prefix("asdf", needle));
        assert!(ci::str_is_prefix("asdf", needle));
    }
}

#[test]
fn str_is_suffix() {
    for needle in ["", "f", "df", "sdf", "asdf"] {
        assert!(cs::str_is_suffix("asdf", needle));
    }
    for needle in ["ff", "aasdf"] {
        assert!(!cs::str_is_suffix("asdf", needle));
    }

    for needle in ["F", "Df"] {
        assert!(!cs::str_is_suffix("asdf", needle));
        assert!(ci::str_is_suffix("asdf", needle));
    }
}

#[test]
fn str_compare_and_is_equal() {
    assert_eq!(cs::str_compare("", ""), Ordering::Equal);
    assert_eq!(cs::str_compare("a", "a"), Ordering::Equal);
    assert_eq!(cs::str_compare("a", "b"), Ordering::Less);
    assert_eq!(cs::str_compare("b", "a"), Ordering::Greater);
    // A prefix sorts before the longer string.
    assert_eq!(cs::str_compare("as", "asdf"), Ordering::Less);
    assert_eq!(cs::str_compare("asdf", "as"), Ordering::Greater);

    assert!(!cs::str_is_equal("asdf", "ASDF"));
    assert!(ci::str_is_equal("asdf", "ASDF"));
    assert_eq!(ci::str_compare("asdf", "ASDF"), Ordering::Equal);
}

#[test]
fn str_matches_glob_with_wildcards() {
    assert!(cs::str_matches_glob("", ""));
    assert!(cs::str_matches_glob("", "*"));
    assert!(!cs::str_matches_glob("", "?"));
    assert!(cs::str_matches_glob("asdf", "asdf"));
    assert!(cs::str_matches_glob("asdf", "*"));
    assert!(cs::str_matches_glob("asdf", "a??f"));
    assert!(!cs::str_matches_glob("asdf", "a?f"));
    assert!(cs::str_matches_glob("asdf", "*f"));
    assert!(cs::str_matches_glob("asdf", "a*f"));
    assert!(cs::str_matches_glob("asdf", "?s?f"));
    assert!(cs::str_matches_glob("asdfjkl", "a*f*l"));
    assert!(cs::str_matches_glob("asdfjkl", "*a*f*l*"));
    assert!(cs::str_matches_glob("asd*fjkl", "*a*f*l*"));
    assert!(!cs::str_matches_glob("classname", "*_color"));
}

#[test]
fn str_matches_glob_respects_case() {
    assert!(!cs::str_matches_glob("asdf", "*F"));
    assert!(!cs::str_matches_glob("asdF", "a*f"));
    assert!(!cs::str_matches_glob("ASDF", "?S?f"));

    assert!(ci::str_matches_glob("asdf", "*F"));
    assert!(ci::str_matches_glob("asdF", "a*f"));
    assert!(ci::str_matches_glob("ASDF", "?S?f"));
}

#[test]
fn str_matches_glob_with_escapes() {
    assert!(cs::str_matches_glob("asd*fjkl", "asd\\*fjkl"));
    assert!(cs::str_matches_glob("asd*?fj\\kl", "asd\\*\\?fj\\\\kl"));
    assert!(cs::str_matches_glob("34dkadj%773", "*\\%%*"));
}

#[test]
fn str_matches_glob_with_digit_wildcards() {
    // `%` is exactly one digit.
    assert!(!cs::str_matches_glob("", "%"));
    for digit in ["0", "1", "2", "9"] {
        assert!(cs::str_matches_glob(digit, "%"));
    }
    assert!(!cs::str_matches_glob("99", "%"));
    assert!(!cs::str_matches_glob("a", "%"));
    assert!(!cs::str_matches_glob("Z", "%"));

    // `%*` is any run of digits, including none.
    assert!(cs::str_matches_glob("", "%*"));
    assert!(!cs::str_matches_glob("3Z", "%*"));

    assert!(!cs::str_matches_glob("Zasdf", "*%"));
    assert!(cs::str_matches_glob("Zasdf3", "*%"));
    assert!(cs::str_matches_glob("Zasdf33", "*%"));
    assert!(cs::str_matches_glob("Zasdf33", "Z*%%"));
    assert!(cs::str_matches_glob("Zasdf3376", "Z*%*"));
    assert!(cs::str_matches_glob("Zasdf3376bdc", "Z*%*"));
    assert!(!cs::str_matches_glob("Zasdf3376bdc", "Zasdf%*"));
    assert!(cs::str_matches_glob("Zasdf3376bdc", "Z*%*bdc"));
    assert!(cs::str_matches_glob("Zasdf3376bdc", "Z*%**"));
    assert!(cs::str_matches_glob("78777Zasdf3376bdc", "%*Z*%**"));
}
