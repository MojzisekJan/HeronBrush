# Translating the manual

The built-in manual is `docs/manual/en.md`, translated into all 29 other UI languages; each file is named after the UI language code (`zh-TW.md` for Traditional Chinese, `pt.md` for Brazilian Portuguese). Until a language has a file, the editor shows the English manual.

## Rules

1. **Same structure as `en.md`.** Every `# ` chapter and every `## ` section must be there, in the same order. Translate the headings, but do not add, drop, merge or split them. Do not use `###` (the reader shows it as plain text), tables, links or images. One paragraph is one line; list items start with `- `, numbered steps with `1. `.
2. **UI labels exactly as the UI shows them.** When the manual names a menu, button, tab or setting, copy the label from your language's translation table, character for character, so readers can find it on screen. The tables are in `crates/editor_core/src/i18n/`, one file per language (`de.rs`, `pt_br.rs`, ...).

   Each entry is `("English label", "translation")`; search for the English label used in `en.md`. A key with a context, such as `("colour scheme|Light", "...")`, is shown without the part before the `|`. If a label has no entry, the UI shows it in English, so write it in English too (and consider adding the missing entry to every table).
3. **Leave code untouched.** Everything between backticks stays exactly as in `en.md`: shortcuts (`Ctrl+Shift+Z`, `F11`), keys, command-line flags (`--merge-map`), file names and paths, entity class names and property keys (`func_button`, `targetname`). Translate only the text around them. Keep `**bold**` markers around labels.
4. Write plain, friendly, concise prose, as the English does. Describe the editor; no marketing.

## Registering the file

Add one line to `MANUALS` in `crates/editor_core/src/manual.rs`, with the search placeholder and the "nothing found" text in your language:

```
Manual { language: Language::German, text: include_str!("../../../docs/manual/de.md"), search: "Dokumentation durchsuchen", nothing: "Keine passenden Kapitel" },
```

The editor then offers the language in the manual's language list and opens it by default when it is the UI language.

## Checking

```
cargo test -p editor_core --lib manual
```

`every_manual_translation_matches_the_english_structure` fails if the number of chapters or sections differs from `en.md`, or if a shortcut or command-line flag written as code in `en.md` is missing from the translation. When `en.md` changes, update every translation in the same commit so the test keeps passing.
