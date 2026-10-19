# UI translations

English is the source language. The other 29 languages use the tables in
`crates/editor_core/src/i18n/`, one file per language (`de.rs`, `pt_br.rs`, ...),
registered in `crates/editor_core/src/i18n.rs`. Each table must contain the same keys, sorted
without duplicates. Missing translations fall back to English at runtime.

Use `tr("Label")` for a static label and `fill("Connected guests: {}", &[count])`
for a translated pattern. Preserve every `{}` placeholder in each translation.
Use `counted` for nouns with plural forms instead of joining translated fragments.
Product names and technical identifiers generally keep their original spelling.

When a label is stored in an array and translated later, mark it with
`translation_key("Label")`. This returns the English key unchanged, so the
language is still chosen when `tr(label)` displays it. It also lets the source
coverage test discover the key. Do not store an already translated value as a
persistent identifier or entity property.

Run:

```sh
cargo test -p editor_core --lib --offline i18n::tests -- --test-threads=1
```

The tests check table consistency, nonempty translations, placeholders, plural
forms and literal keys in `tr`, `fill` and `translation_key` calls throughout the
frontend. Rust token parsing includes calls inside macros and handles escaped or
raw string literals. The i18n module itself is excluded because its tests include
intentional fallback keys. Computed strings and unmarked table entries still
need code review; this is not a linguistic review or a visual layout check.

The latest additions cover render cameras and depth of field, live sessions,
export controls, common dialogs and previously untranslated tooltips for groups,
materials and Godot live linking. All nine translation tables have 658 entries. French and Brazilian Portuguese
are available in the language chooser and detected from `fr` and `pt` system
locales (including regional variants). Portuguese locales currently use the
Brazilian translation. Integer counts of zero and one use the singular in both
new languages. Automated tests cover locale resolution, plural forms, translation
keys and placeholders; a native-speaker review is still useful for terminology.
