//! English (`en`): the editor's own text, for [`super::LANGUAGES`].
//!
//! See `README.md` beside this file for how to translate it.

use super::PluralRule;

/// Which form of a counted noun a number takes: one; other (0, 2, 3, ...).
pub(super) const PLURAL: PluralRule = PluralRule::OneOther;

/// The nouns [`super::counted`] puts after a number, by their English singular: one form
/// for each of [`PLURAL`]'s categories, in the order given there.
pub(super) const NOUNS: [(&str, &[&str]); 13] = [
    ("brush", &["brush", "brushes"]),
    ("edge", &["edge", "edges"]),
    ("entity", &["entity", "entities"]),
    ("face", &["face", "faces"]),
    ("file", &["file", "files"]),
    ("group", &["group", "groups"]),
    ("issue", &["issue", "issues"]),
    ("material", &["material", "materials"]),
    ("object", &["object", "objects"]),
    ("patch", &["patch", "patches"]),
    ("point", &["point", "points"]),
    ("triangle", &["triangle", "triangles"]),
    ("vertex", &["vertex", "vertices"]),
];

/// English is what the source is written in, so it needs no table: a string is its own
/// translation.
pub(super) const TABLE: &[(&str, &str)] = &[];
