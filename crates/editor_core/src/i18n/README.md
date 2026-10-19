# Translating Heron Brush

Every language the editor speaks is one file here, `<code>.rs`, and one line of
`LANGUAGES` in `../i18n.rs`. English is the source: a string a language does not
translate shows in English.

## A language's file

```rust
//! Russian (`ru`): the editor's own text, for [`super::LANGUAGES`].
//!
//! See `README.md` beside this file for how to translate it.

use super::PluralRule;

/// Which form of a counted noun a number takes: one (1, 21, 31, ... but not 11); few (2-4, 22-24, ... but not 12-14); other (0, 5-20, 25-30, ...).
pub(super) const PLURAL: PluralRule = PluralRule::EastSlavic;

/// The nouns [`super::counted`] puts after a number, by their English singular: one form
/// for each of [`PLURAL`]'s categories, in the order given there.
pub(super) const NOUNS: [(&str, &[&str]); 13] = [
    ("brush", &["браш", "браша", "брашей"]),
    // ... edge, entity, face, file, group, issue, material, object, patch, point, triangle, vertex
];

/// The Russian for each of the editor's own strings, sorted by the English.
pub(super) const TABLE: &[(&str, &str)] = &[
    (" (+{} more selected)", " (ещё {} выбрано)"),
    // ...
];
```

- `TABLE` maps each English string to its translation, **sorted by the English**
  (byte order, as the list below prints it) with no repeats.
- `NOUNS` are the thirteen nouns the editor counts ("3 faces"). Keep the English keys and
  their order; give exactly as many forms as `PLURAL` has categories, in the order its
  comment lists them. `counted()` writes `"{number} {form}"`, so a language that puts a
  measure word or a preposition after the number includes it in the form (Chinese
  `"个面"`, Romanian `"de fețe"` in the "other" form).
- `PLURAL` is set already for each language; see `PluralRule` in `../i18n.rs` if one
  ever needs changing.

## How to translate

1. List what is missing, ready to paste into `TABLE`:

   ```sh
   HERON_LANG=ru cargo test -p editor_core --lib i18n::missing -- --ignored --nocapture
   ```

   Without `HERON_LANG` it prints how many strings each language still lacks. The
   lines come out as `("English", ""),` in the right order; fill in the second string.
   It also lists strings a table has that nothing asks for any more (remove them) and
   counted nouns still in English.
2. Translate, keeping:
   - every `{}` placeholder, exactly as many as the English has (the tests check the
     count); move them where your language wants the value;
   - key names and shortcuts (`Ctrl`, `Shift`, `Alt`, `Tab`, `F2`, `Ctrl+Z` …), command
     line flags (anything starting `--`), file extensions and formats (`.map`, `.glb`, `.obj`,
     `.tscn`), program and engine names (Quake, Godot, Blender) and symbols
     such as `⟲ 90°` as they are;
   - leading and trailing spaces and punctuation the English has (`" (+{} more
     selected)"` starts with a space because it is appended to another string).
3. Use the terminology your language's 3D and level-design software already uses —
   Blender's and Godot's localisations are good references — and keep mapper jargon the
   community keeps in English (many keep "brush", "patch", "UV").
4. Translate the thirteen `NOUNS` and remove their `// TODO translate` marks.
5. When nothing is missing, set `complete: true` on the language's line in `LANGUAGES`
   (`../i18n.rs`). Until then it is hidden from the language choosers and not chosen
   for the desktop's locale, though it can be tried by writing `ui/Language=<stored
   name>` (e.g. `russian`) into the preferences file.
6. Run the checks; a complete language is held to every string:

   ```sh
   cargo test -p editor_core --lib i18n
   ```

## Strings the source passes through a variable

The tests find the strings to translate by reading the source for the literal
arguments of `tr`, `tr_text`, `fill`, `tr_ctx` and `translation_key` (and every literal
inside a `tr(if … { "A" } else { "B" })` or `tr(match …)`). A label that reaches `tr`
through a variable -- an enum's `name()`, a const array of labels, a closure's argument
-- is written as `translation_key("Label")` where it starts, or the scan never sees it
and it stays English in every language.

## One English word, two meanings

When the same English needs different translations in different places, the key carries
a context before a `|`: `"colour scheme|Light"` (light, the opposite of dark) is not
`"Light"` (the renderer's lighting). Write `tr_ctx("colour scheme", "Light")` at the
call, or `translation_key("colour scheme|Light")` in a table passed to `tr` later; both
show the text after the `|` when a language has no translation. Translate only the text,
never the context. The contexts in use: `colour scheme`, `language`, `tool material`.

## Adding a language

1. A variant in `enum Language` (`../i18n.rs`).
2. A line in `LANGUAGES`: variant, BCP 47 code, the word the preferences file stores
   (never change one that has shipped), the name in the language itself, the locale
   prefixes it is picked for (lower case, `_`), the file's module, `complete: false`.
3. `mod <code>;` in `../i18n.rs` and the file, copied from one with the same plural rule
   with `TABLE` emptied.

The language choosers in both frontends, the agent settings tool, the preferences file
and locale matching all read `LANGUAGES`; nothing else needs to change. A script the
bundled Inter font lacks (Inter has Latin, Greek, Cyrillic and Vietnamese) needs a
system fallback in `heron-brush/src/theme.rs` (`FALLBACK_FONTS`).
