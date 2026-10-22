//! Tests for the brush font.

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;
use map_model::text;

fn world() -> Bbox3d {
    Bbox3d::from_scalars(-16384.0, 16384.0)
}

const HEIGHT: f64 = 64.0;
const DEPTH: f64 = 16.0;

fn brushes(text: &str) -> Vec<map_model::brush::Brush> {
    text::text_brushes(text, HEIGHT, DEPTH, "stone", Vec3d::zero(), &world())
}

/// Every printable character the font knows.
const KNOWN: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 -_=+/\\.,:!?'\"()<>";

#[test]
fn no_two_brushes_of_a_glyph_share_a_volume() {
    for c in KNOWN.chars() {
        let glyph = brushes(&c.to_string());
        for (i, a) in glyph.iter().enumerate() {
            for b in &glyph[i + 1..] {
                assert!(
                    !a.intersects(b),
                    "two brushes of '{c}' overlap:\n  {:?}\n  {:?}",
                    a.bounds(),
                    b.bounds()
                );
            }
        }
    }
}

#[test]
fn every_letter_and_digit_makes_at_least_two_brushes() {
    for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars() {
        assert!(
            brushes(&c.to_string()).len() >= 2,
            "'{c}' came out as fewer than two segments"
        );
    }
}

#[test]
fn characters_the_font_lacks_are_skipped() {
    assert_eq!(brushes("H\u{2603}I").len(), brushes("HI").len());
    assert!(brushes("\u{2603}\u{2604}").is_empty());
    assert!(brushes("").is_empty());
    assert!(brushes("   ").is_empty(), "a space lights no segments");
}

#[test]
fn a_word_is_the_sum_of_its_letters() {
    let h = brushes("H").len();
    let i = brushes("I").len();
    assert_eq!(brushes("HI").len(), h + i);
}

#[test]
fn the_text_sits_at_the_origin_and_is_the_height_it_was_asked_for() {
    let word = brushes("AB");
    let bounds = word
        .iter()
        .map(|brush| brush.bounds())
        .reduce(|acc, next| acc.merged(&next))
        .expect("some brushes");

    assert!(bounds.min.z() >= -1.0, "{:?}", bounds.min);
    assert!(
        (bounds.max.z() - HEIGHT).abs() < HEIGHT * 0.05,
        "{:?}",
        bounds.max
    );
    assert!(bounds.min.x() >= -1.0, "{:?}", bounds.min);
    assert!(
        bounds.min.y() >= -1.0 && (bounds.max.y() - DEPTH).abs() < 1.0,
        "{:?}",
        bounds
    );
}

#[test]
fn a_wider_string_reports_a_wider_width() {
    assert!(text::text_width("HELLO", HEIGHT) > text::text_width("HI", HEIGHT));
    assert_eq!(text::text_width("", HEIGHT), 0.0);

    let word = brushes("HELLO");
    let right = word
        .iter()
        .map(|brush| brush.bounds().max.x())
        .fold(0.0_f64, f64::max);
    assert!((right - text::text_width("HELLO", HEIGHT)).abs() < HEIGHT * 0.3);
}

#[test]
fn a_newline_starts_a_lower_line() {
    let two_lines = brushes("H\nH");
    let bounds = two_lines
        .iter()
        .map(|brush| brush.bounds())
        .reduce(|acc, next| acc.merged(&next))
        .expect("some brushes");
    assert!(bounds.size().z() > HEIGHT * 1.5);
}

#[test]
fn lowercase_is_drawn_and_some_of_it_differs_from_the_capital() {
    // Every lowercase letter makes brushes.
    for c in "abcdefghijklmnopqrstuvwxyz".chars() {
        assert!(!brushes(&c.to_string()).is_empty(), "'{c}' made nothing");
    }
    // And at least the clearly-lowercase ones are not just the capital.
    for c in "aehnrt".chars() {
        let lower = brushes(&c.to_string()).len();
        let upper = brushes(&c.to_ascii_uppercase().to_string()).len();
        assert_ne!(lower, upper, "'{c}' is identical to its capital");
    }
}

#[test]
fn the_font_is_monospaced() {
    // Every glyph advances the same, so a five-letter word is five glyph-widths wide
    // whatever the letters are.
    assert_eq!(
        text::text_width("11111", HEIGHT),
        text::text_width("WWWWW", HEIGHT)
    );
}
