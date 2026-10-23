//! What language the editor speaks.
//!
//! Looks up English source strings in translation tables. Missing entries fall back to the source
//! text.
//!
//! Both the key and the translation are `'static`, so a lookup gives back a `&'static str`
//! and wrapping a label in [`tr`] changes nothing about how it is passed on.
//!
//! What is read at startup stays read: a language chosen in the preferences reaches
//! windows built after it, which in practice means reopening the window -- the editor's
//! own strings are built once, when its window is.
//!
//! Each language lives in its own file beside this one (`i18n/<code>.rs`), with its table,
//! its counted nouns and its plural rule, and is named once in [`LANGUAGES`]. Adding a
//! language is a variant of [`Language`], a line there and its file; `i18n/README.md` says
//! how to translate one.

use crate::preferences;

mod bg;
mod cs;
mod da;
mod de;
mod el;
mod en;
mod es;
mod fi;
mod fr;
mod hr;
mod hu;
mod id;
mod it;
mod ja;
mod ko;
mod lt;
mod nb;
mod nl;
mod pl;
mod pt_br;
mod ro;
mod ru;
mod sk;
mod sl;
mod sv;
mod tr;
mod uk;
mod vi;
mod zh;
mod zh_tw;

/// Which language the editor's own text is in.
///
/// Everything else about a language -- its name, code, table and plural rule -- is in its
/// line of [`LANGUAGES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// Whichever the desktop asks for, falling back to English.
    #[default]
    System,
    /// English, the language the source is written in.
    English,
    Czech,
    German,
    /// Simplified Chinese.
    Chinese,
    Spanish,
    Ukrainian,
    Polish,
    Italian,
    French,
    /// Brazilian Portuguese.
    PortugueseBrazil,
    Japanese,
    Korean,
    Turkish,
    Indonesian,
    Russian,
    Dutch,
    Swedish,
    Danish,
    NorwegianBokmal,
    Finnish,
    Hungarian,
    Romanian,
    Slovak,
    Greek,
    Vietnamese,
    /// Traditional Chinese.
    ChineseTraditional,
    Bulgarian,
    Croatian,
    Slovenian,
    Lithuanian,
}

/// How a language picks the form of a noun after a number.
///
/// Each names the categories a number falls into, after CLDR's rules for whole numbers;
/// a language's [`LanguageInfo::nouns`] gives one form per category, in the order listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralRule {
    /// One form for every count: Chinese, Japanese, Korean, Vietnamese, Turkish, Indonesian.
    NoPlural,
    /// One (1); other (0, 2, 3, ...): English, German and most of Western Europe.
    OneOther,
    /// One (0 and 1); other (2, 3, ...): French and Brazilian Portuguese.
    French,
    /// One (1); few (2-4); other (0, 5, ...): Czech and Slovak.
    OneFewMany,
    /// One (1, 21, 31 ... not 11); few (2-4, 22-24 ... not 12-14); other: Ukrainian,
    /// Russian, Belarusian, and Croatian, Serbian and Bosnian, which count the same way.
    EastSlavic,
    /// One (1 only); few (2-4, 22-24 ... not 12-14); other: Polish.
    Polish,
    /// One (1, 101 ...); two (2, 102 ...); few (3-4, 103-104 ...); other: Slovenian.
    Slovenian,
    /// One (1, 21 ... not 11); few (2-9, 22-29 ... not 12-19); other (0, 10-20, 30 ...):
    /// Lithuanian.
    Lithuanian,
    /// One (1); few (0, 2-19, 101-119 ...); other (20-100, 120 ...): Romanian, whose
    /// "other" form usually takes "de".
    Romanian,
}

impl PluralRule {
    /// How many forms each noun has under this rule.
    pub const fn forms(self) -> usize {
        match self {
            PluralRule::NoPlural => 1,
            PluralRule::OneOther | PluralRule::French => 2,
            PluralRule::OneFewMany
            | PluralRule::EastSlavic
            | PluralRule::Polish
            | PluralRule::Lithuanian
            | PluralRule::Romanian => 3,
            PluralRule::Slovenian => 4,
        }
    }

    /// Which of the forms `count` takes, counting from zero.
    pub fn form(self, count: usize) -> usize {
        let (last, last_two) = (count % 10, count % 100);
        // The middle form of the Slavic rules: a last digit of two to four, except for
        // the teens.
        let paucal = matches!(last, 2..=4) && !matches!(last_two, 12..=14);
        match self {
            PluralRule::NoPlural => 0,
            PluralRule::OneOther => usize::from(count != 1),
            PluralRule::French => usize::from(count > 1),
            PluralRule::OneFewMany => match count {
                1 => 0,
                2..=4 => 1,
                _ => 2,
            },
            PluralRule::EastSlavic if last == 1 && last_two != 11 => 0,
            PluralRule::EastSlavic | PluralRule::Polish if paucal => 1,
            PluralRule::EastSlavic => 2,
            PluralRule::Polish => usize::from(count != 1) * 2,
            PluralRule::Slovenian => match last_two {
                1 => 0,
                2 => 1,
                3 | 4 => 2,
                _ => 3,
            },
            PluralRule::Lithuanian if matches!(last_two, 11..=19) => 2,
            PluralRule::Lithuanian => match last {
                1 => 0,
                2..=9 => 1,
                _ => 2,
            },
            PluralRule::Romanian if count == 1 => 0,
            PluralRule::Romanian if count == 0 || matches!(last_two, 1..=19) => 1,
            PluralRule::Romanian => 2,
        }
    }
}

/// One language the editor can speak: one line of [`LANGUAGES`].
#[derive(Debug)]
pub struct LanguageInfo {
    pub language: Language,
    /// The BCP 47 code, as agents name it: `cs`, `pt-BR`, `zh-TW`.
    pub code: &'static str,
    /// How the preferences file writes it (`ui/Language=czech`). Never change one that has
    /// shipped, or the choice is lost when the file is read.
    pub stored: &'static str,
    /// The name shown in the preferences, in the language it names.
    pub name: &'static str,
    /// The locales it is chosen for when following the desktop, lower case with `_`:
    /// `"zh"` matches `zh_CN.UTF-8` and `zh-Hans`, and the longest match wins, so
    /// `"zh_tw"` takes `zh_TW` from it.
    pub locales: &'static [&'static str],
    /// Whether every string is translated. An incomplete language is not offered in the
    /// preferences or picked for the desktop's locale, and what it lacks shows in English;
    /// a complete one is held to every string by the tests.
    pub complete: bool,
    /// English to the translation, sorted by the English.
    pub table: &'static [(&'static str, &'static str)],
    /// How [`counted`] picks one of a noun's forms.
    pub plural: PluralRule,
    /// The nouns [`counted`] knows, by their English singular, each with
    /// [`PluralRule::forms`] forms.
    pub nouns: &'static [(&'static str, &'static [&'static str]); 17],
}

/// One line of [`LANGUAGES`]: the variant, code, stored name, native name, locales, file
/// and whether it is complete. The table, nouns and plural rule come from the file.
macro_rules! language {
    ($language:ident, $code:literal, $stored:literal, $name:literal, [$($locale:literal),*], $file:ident, complete: $complete:literal) => {
        LanguageInfo {
            language: Language::$language,
            code: $code,
            stored: $stored,
            name: $name,
            locales: &[$($locale),*],
            complete: $complete,
            table: $file::TABLE,
            plural: $file::PLURAL,
            nouns: &$file::NOUNS,
        }
    };
}

/// Every language the editor speaks, English first, in the order the chooser lists them.
pub const LANGUAGES: &[LanguageInfo] = &[
    language!(English, "en", "english", "English", ["en"], en, complete: true),
    language!(Czech, "cs", "czech", "Čeština", ["cs"], cs, complete: true),
    language!(German, "de", "german", "Deutsch", ["de"], de, complete: true),
    language!(Chinese, "zh", "chinese", "中文", ["zh"], zh, complete: true),
    language!(Spanish, "es", "spanish", "Español", ["es"], es, complete: true),
    language!(Ukrainian, "uk", "ukrainian", "Українська", ["uk"], uk, complete: true),
    language!(Polish, "pl", "polish", "Polski", ["pl"], pl, complete: true),
    language!(Italian, "it", "italian", "Italiano", ["it"], it, complete: true),
    language!(French, "fr", "french", "Français", ["fr"], fr, complete: true),
    language!(PortugueseBrazil, "pt-BR", "portuguese_brazil", "Português (Brasil)", ["pt"], pt_br, complete: true),
    language!(Japanese, "ja", "japanese", "日本語", ["ja"], ja, complete: true),
    language!(Korean, "ko", "korean", "한국어", ["ko"], ko, complete: true),
    language!(Turkish, "tr", "turkish", "Türkçe", ["tr"], tr, complete: true),
    language!(Indonesian, "id", "indonesian", "Bahasa Indonesia", ["id", "in"], id, complete: true),
    language!(Russian, "ru", "russian", "Русский", ["ru"], ru, complete: true),
    language!(Dutch, "nl", "dutch", "Nederlands", ["nl"], nl, complete: true),
    language!(Swedish, "sv", "swedish", "Svenska", ["sv"], sv, complete: true),
    language!(Danish, "da", "danish", "Dansk", ["da"], da, complete: true),
    language!(NorwegianBokmal, "nb", "norwegian_bokmal", "Norsk bokmål", ["nb", "no", "nn"], nb, complete: true),
    language!(Finnish, "fi", "finnish", "Suomi", ["fi"], fi, complete: true),
    language!(Hungarian, "hu", "hungarian", "Magyar", ["hu"], hu, complete: true),
    language!(Romanian, "ro", "romanian", "Română", ["ro"], ro, complete: true),
    language!(Slovak, "sk", "slovak", "Slovenčina", ["sk"], sk, complete: true),
    language!(Greek, "el", "greek", "Ελληνικά", ["el"], el, complete: true),
    language!(Vietnamese, "vi", "vietnamese", "Tiếng Việt", ["vi"], vi, complete: true),
    language!(ChineseTraditional, "zh-TW", "chinese_traditional", "繁體中文", ["zh_tw", "zh_hk", "zh_mo", "zh_hant"], zh_tw, complete: true),
    language!(Bulgarian, "bg", "bulgarian", "Български", ["bg"], bg, complete: true),
    language!(Croatian, "hr", "croatian", "Hrvatski", ["hr"], hr, complete: true),
    language!(Slovenian, "sl", "slovenian", "Slovenščina", ["sl"], sl, complete: true),
    language!(Lithuanian, "lt", "lithuanian", "Lietuvių", ["lt"], lt, complete: true),
];

impl Language {
    /// Following the desktop, then every language in [`LANGUAGES`], finished or not.
    pub fn all() -> impl Iterator<Item = Language> {
        std::iter::once(Language::System).chain(LANGUAGES.iter().map(|info| info.language))
    }

    /// What the preferences offer: following the desktop and every complete language,
    /// plus `current` if it is one still being translated, so a choice made by hand in
    /// the preferences file still shows.
    pub fn choices(current: Language) -> Vec<Language> {
        let mut choices: Vec<Language> = Language::all()
            .filter(|language| *language == Language::System || language.info().complete)
            .collect();
        if !choices.contains(&current) {
            choices.push(current);
        }
        choices
    }

    /// The language's line in [`LANGUAGES`]; English's for [`Language::System`], which
    /// has none.
    pub fn info(self) -> &'static LanguageInfo {
        LANGUAGES
            .iter()
            .find(|info| info.language == self)
            .unwrap_or(&LANGUAGES[0])
    }

    /// The language with this [`LanguageInfo::code`], or `"system"`.
    pub fn from_code(code: &str) -> Option<Language> {
        if code == "system" {
            return Some(Language::System);
        }
        LANGUAGES
            .iter()
            .find(|info| info.code == code)
            .map(|info| info.language)
    }

    /// [`LanguageInfo::code`], or `"system"`.
    pub fn code(self) -> &'static str {
        match self {
            Language::System => "system",
            language => language.info().code,
        }
    }

    /// The name shown in the preferences, in the language it names; following the
    /// desktop is named in the language in force.
    pub fn name(self) -> &'static str {
        match self {
            Language::System => tr_ctx("language", "Follow the desktop"),
            language => language.info().name,
        }
    }

    /// How the preferences file writes it.
    pub fn stored(self) -> &'static str {
        match self {
            Language::System => "system",
            language => language.info().stored,
        }
    }

    /// The language the preferences file wrote as `text`.
    pub fn from_stored(text: &str) -> Option<Language> {
        if text == "system" {
            return Some(Language::System);
        }
        LANGUAGES
            .iter()
            .find(|info| info.stored == text)
            .map(|info| info.language)
    }

    /// Match a locale against [`LanguageInfo::locales`], independent of encoding,
    /// modifier, separator and case; the longest match wins and none is English.
    #[cfg(test)]
    fn from_locale(locale: &str) -> Self {
        Self::from_locale_among(locale, false)
    }

    /// [`Language::from_locale`], among the complete languages only when `complete`.
    fn from_locale_among(locale: &str, complete: bool) -> Self {
        let locale = locale
            .split(['.', '@'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .replace('-', "_");
        LANGUAGES
            .iter()
            .filter(|info| info.complete || !complete)
            .flat_map(|info| {
                info.locales
                    .iter()
                    .map(move |prefix| (info.language, *prefix))
            })
            .filter(|(_, prefix)| {
                locale
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
            })
            .max_by_key(|(_, prefix)| prefix.len())
            .map_or(Language::English, |(language, _)| language)
    }

    /// The language to actually speak: what the desktop asks for, when that is what was
    /// chosen. Never [`Language::System`].
    pub fn resolved(self) -> Language {
        match self {
            Language::System => {
                // `LC_ALL`, `LC_MESSAGES`, then `LANG`, which is the order gettext reads
                // them in. A language still being translated is passed over -- `zh_TW`
                // reads Simplified Chinese until Traditional is complete -- and anything
                // else is English, since English is what the source is written in.
                let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
                    .iter()
                    .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
                    .unwrap_or_default();
                Self::from_locale_among(&locale, true)
            }
            named => named,
        }
    }
}

/// The line of [`LANGUAGES`] for the language in force.
fn speaking() -> &'static LanguageInfo {
    preferences::pref(|p| p.language).resolved().info()
}

/// Mark a static key in a table for translation at display time and coverage checks.
///
/// A string that only reaches [`tr`] through a variable -- an enum's `name()`, a const
/// array of labels -- is wrapped in this where it is written, so the coverage scan finds
/// it and every complete language must translate it.
pub const fn translation_key(text: &'static str) -> &'static str {
    text
}

/// What separates a key's context from its text: `"colour scheme|Light"`.
const CONTEXT: char = '|';

/// The text of a key, without the context [`tr_ctx`] or a `"context|text"`
/// [`translation_key`] gave it.
fn without_context(key: &str) -> &str {
    key.split_once(CONTEXT).map_or(key, |(_, text)| text)
}

/// [`tr`] for an English word that needs translating differently in different places:
/// the colour scheme's "Light" is not the renderer's "Light". The tables hold it as
/// `"context|text"`, and the English shows the text alone.
///
/// Where the string is written into a table and translated later, write the whole key,
/// `translation_key("colour scheme|Light")`, and [`tr`] drops the context the same way.
pub fn tr_ctx(context: &'static str, text: &'static str) -> &'static str {
    look_up(speaking().table, &format!("{context}{CONTEXT}{text}")).unwrap_or(text)
}

/// The translation of `text` in `table`, if it has one.
fn look_up(table: &'static [(&'static str, &'static str)], text: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(english, _)| *english == text)
        .map(|(_, translated)| *translated)
}

/// The text in the language in force, or the English it was written in.
pub fn tr(text: &'static str) -> &'static str {
    look_up(speaking().table, text).unwrap_or_else(|| without_context(text))
}

/// [`tr`] for text that arrives at run time, such as an error message the core wrote
/// with a [`translation_key`]. Text the tables do not know is returned as it is.
pub fn tr_text(text: &str) -> String {
    look_up(speaking().table, text)
        .unwrap_or_else(|| without_context(text))
        .to_owned()
}

/// A count with the noun it counts, in the language's own form.
///
/// English counts two ways, Czech three -- one, two to four, and five and up -- and
/// Chinese one, with a measure word in front of the noun; each language's
/// [`PluralRule`] says which. The noun is named by its English singular; one the
/// language does not know is counted the English way, which is what the source is
/// written in.
pub fn counted(count: usize, noun: &str) -> String {
    let info = speaking();
    let Some((_, forms)) = info.nouns.iter().find(|(key, _)| *key == noun) else {
        return format!("{count} {noun}");
    };
    let form = forms
        .get(info.plural.form(count))
        .or(forms.last())
        .copied()
        .unwrap_or(noun);
    format!("{count} {form}")
}

/// A translated pattern with its `{}` filled in, left to right.
///
/// The pattern is a string in the tables like `"{} to paint"`, so the translation can put
/// the number where its own language wants it.
pub fn fill(pattern: &'static str, values: &[&str]) -> String {
    let mut out = tr(pattern).to_string();
    for value in values {
        if let Some(at) = out.find("{}") {
            out.replace_range(at..at + 2, value);
        }
    }
    out
}

/// Every string the editor's source asks to have translated, in the core and both
/// frontends: the literal arguments of `tr`, `tr_text`, `fill` and `translation_key`;
/// every literal inside a `tr(...)` whose argument picks one (`tr(if on { "A" } else {
/// "B" })`, a `match`); and `tr_ctx("context", "text")` as `"context|text"`.
///
/// A string that reaches `tr` through a variable is invisible here, which is why it is
/// written as a `translation_key` where it starts.
#[cfg(test)]
fn source_keys() -> std::collections::BTreeSet<String> {
    use proc_macro2::{TokenStream, TokenTree};
    use std::{collections::BTreeSet, path::Path};
    fn string(token: &TokenTree) -> Option<String> {
        let TokenTree::Literal(literal) = token else {
            return None;
        };
        syn::parse_str::<syn::LitStr>(&literal.to_string())
            .ok()
            .map(|text| text.value())
    }
    /// Every string literal in `stream`, however deep.
    fn every_string(stream: TokenStream, keys: &mut BTreeSet<String>) {
        for token in stream {
            match &token {
                TokenTree::Group(group) => every_string(group.stream(), keys),
                _ => keys.extend(string(&token)),
            }
        }
    }
    fn collect(stream: TokenStream, keys: &mut BTreeSet<String>) {
        let tokens: Vec<_> = stream.into_iter().collect();
        for (index, token) in tokens.iter().enumerate() {
            if let (TokenTree::Ident(name), Some(TokenTree::Group(args))) =
                (token, tokens.get(index + 1))
            {
                let args: Vec<_> = args.stream().into_iter().collect();
                let first = args.first().and_then(string);
                match name.to_string().as_str() {
                    "tr_ctx" => {
                        if let (Some(context), Some(text)) = (first, args.get(2).and_then(string)) {
                            keys.insert(format!("{context}{CONTEXT}{text}"));
                        }
                    }
                    "tr" if first.is_none() => every_string(args.into_iter().collect(), keys),
                    "tr" | "tr_text" | "fill" | "translation_key" => keys.extend(first),
                    _ => {}
                }
            }
            if let TokenTree::Group(group) = token {
                collect(group.stream(), keys);
            }
        }
    }
    fn scan(path: &Path, keys: &mut BTreeSet<String>) {
        for item in std::fs::read_dir(path).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                // The tables themselves, which only hold the strings.
                if path.file_name().unwrap() != "i18n" {
                    scan(&path, keys);
                }
            } else if path.extension().is_some_and(|e| e == "rs")
                && path.file_name().unwrap() != "i18n.rs"
            {
                let source = std::fs::read_to_string(&path).unwrap();
                collect(source.parse().expect("valid Rust tokens"), keys);
            }
        }
    }
    // The strings live in the core and in each frontend, and one in this file, which the
    // scan leaves out for its tests' sake: what following the desktop is called.
    let mut keys = BTreeSet::from([format!("language{CONTEXT}Follow the desktop")]);
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for frontend in ["editor_core", "heron-brush"] {
        scan(&crates.join(frontend).join("src"), &mut keys);
    }
    keys
}

/// Every string a translation needs: the source's, and the ones the complete tables
/// share, which the source passes on through variables the scan cannot follow.
#[cfg(test)]
fn wanted_keys() -> std::collections::BTreeSet<String> {
    let mut keys = source_keys();
    keys.extend(cs::TABLE.iter().map(|(english, _)| (*english).to_owned()));
    keys
}

/// The translator's list: `HERON_LANG=ru cargo test -p editor_core --lib i18n::missing --
/// --ignored --nocapture` prints the strings `ru` lacks, ready to paste into `i18n/ru.rs`,
/// and any it has that are no longer wanted. Without `HERON_LANG`, a count per language.
#[cfg(test)]
mod missing {
    use super::*;

    #[test]
    #[ignore = "a tool for translators; prints, never fails"]
    fn keys() {
        let wanted = wanted_keys();
        let missing = |info: &LanguageInfo| -> Vec<&String> {
            wanted
                .iter()
                .filter(|key| look_up(info.table, key).is_none())
                .collect()
        };
        let Some(code) = std::env::var("HERON_LANG")
            .ok()
            .filter(|code| !code.is_empty())
        else {
            println!(
                "{} strings to translate. Missing per language:",
                wanted.len()
            );
            for info in &LANGUAGES[1..] {
                let state = if info.complete {
                    "complete"
                } else {
                    "in progress"
                };
                println!(
                    "  {:<6} {:<20} {:>5}  ({state})",
                    info.code,
                    info.name,
                    missing(info).len()
                );
            }
            println!("Set HERON_LANG=<code> to list one language's missing strings.");
            return;
        };
        let Some(info) = LANGUAGES.iter().find(|info| {
            info.code.eq_ignore_ascii_case(&code)
                || info.code.replace('-', "_").eq_ignore_ascii_case(&code)
        }) else {
            panic!("no language with the code {code:?}; the codes are in i18n::LANGUAGES");
        };
        let missing = missing(info);
        println!(
            "// {} of {} strings missing from {} ({}):",
            missing.len(),
            wanted.len(),
            info.name,
            info.code
        );
        for key in &missing {
            println!("    ({key:?}, \"\"),");
        }
        let stale: Vec<_> = info
            .table
            .iter()
            .filter(|(key, _)| !wanted.contains(*key))
            .collect();
        if !stale.is_empty() {
            println!("// {} no longer wanted, to remove:", stale.len());
            for (key, _) in stale {
                println!("//  {key:?}");
            }
        }
        let untranslated = info
            .nouns
            .iter()
            .filter(|(noun, forms)| {
                info.language != Language::English && forms.first() == Some(noun)
            })
            .count();
        if untranslated > 0 {
            println!(
                "// {untranslated} of the {} counted nouns (NOUNS) still start with the English word; fine only where it is borrowed.",
                info.nouns.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The languages other than English, which is the source and needs no table.
    fn translations() -> impl Iterator<Item = &'static LanguageInfo> {
        LANGUAGES
            .iter()
            .filter(|info| info.language != Language::English)
    }

    #[test]
    fn asian_and_turkish_locales_and_counts() {
        for (locale, language, save, vertex) in [
            ("ja_JP.UTF-8", Language::Japanese, "保存", "頂点"),
            ("ko-KR", Language::Korean, "저장", "정점"),
            ("tr_TR.UTF-8", Language::Turkish, "Kaydet", "köşe"),
            ("id_ID.UTF-8", Language::Indonesian, "Simpan", "verteks"),
        ] {
            assert_eq!(Language::from_locale(locale), language);
            preferences::edit(|p| p.language = language);
            assert_eq!(tr("Save"), save);
            for count in [0, 1, 2, 12] {
                assert_eq!(counted(count, "vertex"), format!("{count} {vertex}"));
            }
        }
        assert_eq!(Language::from_locale("in_ID"), Language::Indonesian);
        preferences::edit(|p| p.language = Language::English);
    }

    #[test]
    fn new_languages_resolve_locales_and_translate_counts() {
        for locale in ["fr_FR.UTF-8", "fr-CA", "FR_be@euro"] {
            assert_eq!(Language::from_locale(locale), Language::French);
        }
        for locale in ["pt_BR.UTF-8", "pt-BR", "pt", "pt_PT.UTF-8"] {
            assert_eq!(Language::from_locale(locale), Language::PortugueseBrazil);
        }
        assert_eq!(Language::from_locale("C.UTF-8"), Language::English);
        assert_eq!(Language::from_locale("frenchish"), Language::English);
        preferences::edit(|p| p.language = Language::French);
        assert_eq!(tr("Save"), "Enregistrer");
        assert_eq!(counted(0, "vertex"), "0 sommet");
        assert_eq!(counted(1, "vertex"), "1 sommet");
        assert_eq!(counted(2, "vertex"), "2 sommets");
        assert_eq!(
            fill("Connected guests: {}", &["3"]),
            "Invités connectés : 3"
        );
        preferences::edit(|p| p.language = Language::PortugueseBrazil);
        assert_eq!(tr("Save"), "Salvar");
        assert_eq!(counted(0, "vertex"), "0 vértice");
        assert_eq!(counted(1, "vertex"), "1 vértice");
        assert_eq!(counted(2, "vertex"), "2 vértices");
        assert_eq!(
            fill("Connected guests: {}", &["3"]),
            "Convidados conectados: 3"
        );
        preferences::edit(|p| p.language = Language::English);
    }

    #[test]
    fn locales_pick_the_registered_language() {
        for (locale, language) in [
            ("ru_RU.UTF-8", Language::Russian),
            ("nl_BE", Language::Dutch),
            ("sv_SE.UTF-8", Language::Swedish),
            ("da_DK", Language::Danish),
            ("nb_NO.UTF-8", Language::NorwegianBokmal),
            ("no_NO", Language::NorwegianBokmal),
            ("nn_NO.UTF-8", Language::NorwegianBokmal),
            ("fi_FI", Language::Finnish),
            ("hu_HU", Language::Hungarian),
            ("ro_RO", Language::Romanian),
            ("sk_SK", Language::Slovak),
            ("el_GR.UTF-8", Language::Greek),
            ("vi_VN", Language::Vietnamese),
            ("bg_BG", Language::Bulgarian),
            ("hr_HR", Language::Croatian),
            ("sl_SI", Language::Slovenian),
            ("lt_LT", Language::Lithuanian),
            // Traditional where Traditional is written, Simplified everywhere else.
            ("zh_TW.UTF-8", Language::ChineseTraditional),
            ("zh-HK", Language::ChineseTraditional),
            ("zh_MO", Language::ChineseTraditional),
            ("zh-Hant", Language::ChineseTraditional),
            ("zh_Hant_TW", Language::ChineseTraditional),
            ("zh_CN.UTF-8", Language::Chinese),
            ("zh-Hans-HK", Language::Chinese),
            ("zh_SG", Language::Chinese),
            ("zh", Language::Chinese),
            ("cs_CZ.UTF-8", Language::Czech),
            ("de_AT@euro", Language::German),
            ("", Language::English),
            ("C", Language::English),
            ("POSIX", Language::English),
            ("en_GB", Language::English),
            ("zhx", Language::English),
        ] {
            assert_eq!(Language::from_locale(locale), language, "{locale}");
            // Following the desktop passes over one still being translated, to its
            // nearest complete relative or English.
            let followed = Language::from_locale_among(locale, true);
            if language.info().complete {
                assert_eq!(followed, language, "{locale}");
            } else {
                assert!(followed.info().complete, "{locale}");
            }
        }
        if !Language::ChineseTraditional.info().complete {
            assert_eq!(
                Language::from_locale_among("zh_TW.UTF-8", true),
                Language::Chinese
            );
            assert_eq!(
                Language::from_locale_among("ru_RU", true),
                Language::English
            );
        }
    }

    #[test]
    fn the_registry_is_consistent() {
        assert_eq!(LANGUAGES[0].language, Language::English);
        for (index, info) in LANGUAGES.iter().enumerate() {
            for other in &LANGUAGES[..index] {
                assert_ne!(
                    info.language, other.language,
                    "{} is listed twice",
                    info.code
                );
                assert_ne!(info.code, other.code);
                assert_ne!(info.stored, other.stored);
                assert_ne!(info.name, other.name);
                for locale in info.locales {
                    assert!(!other.locales.contains(locale), "{locale} is claimed twice");
                }
            }
            assert_eq!(info.language.info().code, info.code);
            assert_eq!(Language::from_code(info.code), Some(info.language));
            assert_eq!(Language::from_stored(info.stored), Some(info.language));
            assert!(!info.locales.is_empty());
            for locale in info.locales {
                assert_eq!(
                    *locale,
                    locale.to_ascii_lowercase().replace('-', "_"),
                    "{locale}"
                );
            }
        }
        // Every variant but System has a line: a forgotten one would read as English.
        for language in Language::all().skip(1) {
            assert!(LANGUAGES.iter().any(|info| info.language == language));
        }
        assert_eq!(Language::all().count(), LANGUAGES.len() + 1);
    }

    #[test]
    fn only_complete_languages_are_offered() {
        let choices = Language::choices(Language::System);
        assert_eq!(choices[0], Language::System);
        for language in &choices[1..] {
            assert!(language.info().complete);
        }
        assert_eq!(
            choices.len(),
            1 + LANGUAGES.iter().filter(|info| info.complete).count()
        );
        // One chosen by hand in the preferences file still shows as itself.
        if let Some(unfinished) = LANGUAGES.iter().find(|info| !info.complete) {
            assert_eq!(
                Language::choices(unfinished.language).last(),
                Some(&unfinished.language)
            );
        }
    }

    #[test]
    fn an_incomplete_language_falls_back_to_english_string_by_string() {
        let Some(unfinished) = LANGUAGES
            .iter()
            .find(|info| !info.complete && info.table.is_empty())
        else {
            return;
        };
        preferences::edit(|p| p.language = unfinished.language);
        assert_eq!(tr("Save"), "Save");
        assert_eq!(tr_text("Save"), "Save");
        preferences::edit(|p| p.language = Language::English);
    }

    #[test]
    fn ui_translation_keys_are_present_in_every_language() {
        let keys = source_keys();
        assert!(keys.len() > 500, "source scanning must discover the UI");
        for info in translations() {
            let language = info.code;
            if info.complete {
                let missing: Vec<_> = keys
                    .iter()
                    .filter(|key| !info.table.iter().any(|(k, _)| k == key))
                    .collect();
                assert!(
                    missing.is_empty(),
                    "missing {language} translations: {missing:?}"
                );
            }
            for (key, translation) in info.table {
                assert_eq!(
                    key.matches("{}").count(),
                    translation.matches("{}").count(),
                    "placeholder mismatch in {language}: {key}"
                );
            }
        }
    }
    #[test]
    fn the_tables_are_sorted_and_have_no_repeats() {
        // Sorted so a missing string is easy to find, and without repeats because the
        // first of two would win and the second would never be seen.
        for info in translations() {
            let table = info.table;
            let mut sorted: Vec<&str> = table.iter().map(|(english, _)| *english).collect();
            let given = sorted.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, given, "the {} table is out of order", info.code);
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                table.len(),
                "the {} table repeats a string",
                info.code
            );
        }
    }

    #[test]
    fn the_tables_say_the_same_things() {
        let czech: Vec<&str> = cs::TABLE.iter().map(|(english, _)| *english).collect();
        for info in translations() {
            let name = info.name;
            let other: Vec<&str> = info.table.iter().map(|(english, _)| *english).collect();
            if info.complete {
                assert_eq!(
                    czech, other,
                    "Czech and {name} do not know the same strings"
                );
            } else {
                // One being translated may lack strings, but not know ones nobody wants.
                let wanted = wanted_keys();
                let stale: Vec<_> = other.iter().filter(|key| !wanted.contains(**key)).collect();
                assert!(
                    stale.is_empty(),
                    "{name} translates strings nothing asks for: {stale:?}"
                );
            }
        }
    }

    #[test]
    fn nothing_is_left_in_english_by_accident() {
        for info in translations() {
            for (english, translated) in info.table {
                assert!(
                    !translated.is_empty(),
                    "{english} has no {} translation",
                    info.code
                );
            }
        }
    }

    #[test]
    fn every_language_counts_the_same_nouns() {
        let english: Vec<&str> = en::NOUNS.iter().map(|(noun, _)| *noun).collect();
        for info in LANGUAGES {
            let nouns: Vec<&str> = info.nouns.iter().map(|(noun, _)| *noun).collect();
            assert_eq!(nouns, english, "{} counts other nouns", info.code);
            for (noun, forms) in info.nouns {
                assert_eq!(
                    forms.len(),
                    info.plural.forms(),
                    "{} {noun}: one form per category",
                    info.code
                );
                assert!(
                    forms.iter().all(|form| !form.is_empty()),
                    "{} {noun}",
                    info.code
                );
            }
        }
    }

    #[test]
    fn plural_rules_pick_the_category_cldr_gives() {
        let forms = |rule: PluralRule, counts: &[usize]| -> Vec<usize> {
            counts.iter().map(|count| rule.form(*count)).collect()
        };
        let counts = [
            0, 1, 2, 3, 4, 5, 11, 12, 14, 19, 20, 21, 22, 25, 101, 102, 103, 111, 112, 120,
        ];
        assert_eq!(forms(PluralRule::NoPlural, &counts), [0; 20]);
        assert_eq!(
            forms(PluralRule::OneOther, &counts),
            [1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
        );
        assert_eq!(
            forms(PluralRule::French, &counts),
            [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
        );
        assert_eq!(
            forms(PluralRule::OneFewMany, &counts),
            [2, 0, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]
        );
        assert_eq!(
            forms(PluralRule::EastSlavic, &counts),
            [2, 0, 1, 1, 1, 2, 2, 2, 2, 2, 2, 0, 1, 2, 0, 1, 1, 2, 2, 2]
        );
        assert_eq!(
            forms(PluralRule::Polish, &counts),
            [2, 0, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 1, 2, 2, 1, 1, 2, 2, 2]
        );
        assert_eq!(
            forms(PluralRule::Slovenian, &counts),
            [3, 0, 1, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 1, 2, 3, 3, 3]
        );
        assert_eq!(
            forms(PluralRule::Lithuanian, &counts),
            [2, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 0, 1, 1, 0, 1, 1, 2, 2, 2]
        );
        assert_eq!(
            forms(PluralRule::Romanian, &counts),
            [1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1, 1, 1, 2]
        );
        for rule in [
            PluralRule::NoPlural,
            PluralRule::OneOther,
            PluralRule::French,
            PluralRule::OneFewMany,
            PluralRule::EastSlavic,
            PluralRule::Polish,
            PluralRule::Slovenian,
            PluralRule::Lithuanian,
            PluralRule::Romanian,
        ] {
            assert!(
                (0..1000).all(|count| rule.form(count) < rule.forms()),
                "{rule:?}"
            );
        }
    }

    #[test]
    fn the_new_languages_count_by_their_own_rule() {
        for (language, rule) in [
            (Language::Russian, PluralRule::EastSlavic),
            (Language::Croatian, PluralRule::EastSlavic),
            (Language::Slovak, PluralRule::OneFewMany),
            (Language::Slovenian, PluralRule::Slovenian),
            (Language::Lithuanian, PluralRule::Lithuanian),
            (Language::Romanian, PluralRule::Romanian),
            (Language::Vietnamese, PluralRule::NoPlural),
            (Language::ChineseTraditional, PluralRule::NoPlural),
            (Language::Dutch, PluralRule::OneOther),
            (Language::Bulgarian, PluralRule::OneOther),
            (Language::Greek, PluralRule::OneOther),
        ] {
            assert_eq!(language.info().plural, rule, "{language:?}");
        }
    }

    #[test]
    fn every_complete_language_is_offered_and_speaks_its_own() {
        let choices = Language::choices(Language::System);
        assert_eq!(
            choices.len(),
            LANGUAGES.len() + 1,
            "System and every language"
        );
        for info in translations() {
            preferences::edit(|p| p.language = info.language);
            for key in [
                "Save",
                "Target requires keyframes",
                "Select all",
                "Missing target: {}",
            ] {
                assert_ne!(tr(key), key, "{} leaves {key:?} in English", info.code);
            }
            assert_ne!(
                counted(2, "entity"),
                "2 entities",
                "{} counts entities in English",
                info.code
            );
        }
        preferences::edit(|p| p.language = Language::English);
    }

    #[test]
    fn a_count_takes_the_form_the_language_gives_it() {
        // Czech counts three ways: one, two to four, and five and up.
        preferences::edit(|p| p.language = Language::Czech);
        assert_eq!(counted(1, "face"), "1 stěna");
        assert_eq!(counted(3, "face"), "3 stěny");
        assert_eq!(counted(7, "face"), "7 stěn");
        assert_eq!(counted(0, "face"), "0 stěn");
        assert_eq!(counted(3, "entity"), "3 entity");
        assert_eq!(counted(5, "entity"), "5 entit");

        preferences::edit(|p| p.language = Language::German);
        assert_eq!(counted(1, "face"), "1 Fläche");
        assert_eq!(counted(3, "face"), "3 Flächen");

        preferences::edit(|p| p.language = Language::Chinese);
        assert_eq!(counted(3, "face"), "3 个面");

        preferences::edit(|p| p.language = Language::Spanish);
        assert_eq!(counted(1, "face"), "1 cara");
        assert_eq!(counted(3, "face"), "3 caras");

        // Ukrainian and Polish count by the last digits: twenty-two takes the middle
        // form again, but the teens stay in the third.
        preferences::edit(|p| p.language = Language::Ukrainian);
        assert_eq!(counted(1, "face"), "1 грань");
        assert_eq!(counted(3, "face"), "3 грані");
        assert_eq!(counted(7, "face"), "7 граней");
        assert_eq!(counted(12, "face"), "12 граней");
        assert_eq!(counted(21, "face"), "21 грань");
        assert_eq!(counted(22, "face"), "22 грані");

        preferences::edit(|p| p.language = Language::Polish);
        assert_eq!(counted(1, "face"), "1 ściana");
        assert_eq!(counted(3, "face"), "3 ściany");
        assert_eq!(counted(7, "face"), "7 ścian");
        assert_eq!(counted(14, "face"), "14 ścian");
        assert_eq!(counted(21, "face"), "21 ścian");
        assert_eq!(counted(22, "face"), "22 ściany");

        preferences::edit(|p| p.language = Language::Italian);
        assert_eq!(counted(1, "face"), "1 faccia");
        assert_eq!(counted(3, "face"), "3 facce");

        preferences::edit(|p| p.language = Language::English);
        assert_eq!(counted(1, "face"), "1 face");
        assert_eq!(counted(3, "face"), "3 faces");
        assert_eq!(counted(1, "entity"), "1 entity");
        assert_eq!(
            fill("{} of different classes", &[&counted(2, "entity")]),
            "2 entities of different classes"
        );
        // A noun the table does not know is counted the English way.
        assert_eq!(counted(2, "widget"), "2 widget");
    }

    #[test]
    fn a_key_with_context_shows_only_its_text() {
        preferences::edit(|p| p.language = Language::English);
        assert_eq!(tr("colour scheme|Light"), "Light");
        assert_eq!(tr_ctx("colour scheme", "Light"), "Light");
        assert_eq!(tr_text("tool material|Clip"), "Clip");
        assert_eq!(Language::System.name(), "Follow the desktop");
        // The colour scheme's Light is not the renderer's.
        preferences::edit(|p| p.language = Language::Czech);
        assert_eq!(tr("colour scheme|Light"), "Světlé");
        assert_eq!(tr_ctx("colour scheme", "Light"), "Světlé");
        assert_eq!(tr("Light"), "Světlo");
        assert_eq!(Language::System.name(), "Podle systému");
        preferences::edit(|p| p.language = Language::English);
    }

    #[test]
    fn a_pattern_is_filled_in_where_the_translation_wants_it() {
        preferences::edit(|p| p.language = Language::Czech);
        assert_eq!(fill("{} to paint", &["3 stěny"]), "3 stěny k obarvení");
        preferences::edit(|p| p.language = Language::English);
        assert_eq!(fill("{} to paint", &["3 faces"]), "3 faces to paint");
        // More values than the pattern has room for is not an error: the rest are left.
        assert_eq!(fill("{} to paint", &["one", "two"]), "one to paint");
    }

    #[test]
    fn a_string_the_table_does_not_have_is_left_as_it_is() {
        preferences::edit(|p| p.language = Language::Czech);
        assert_eq!(tr("Vertex"), "Vrchol");
        assert_eq!(tr("__TB_empty"), "__TB_empty");
        preferences::edit(|p| p.language = Language::German);
        assert_eq!(tr("Vertex"), "Eckpunkt");
        preferences::edit(|p| p.language = Language::Chinese);
        assert_eq!(tr("Vertex"), "顶点");
        assert_eq!(tr("__TB_empty"), "__TB_empty");
        preferences::edit(|p| p.language = Language::Spanish);
        assert_eq!(tr("Vertex"), "Vértice");
        preferences::edit(|p| p.language = Language::Ukrainian);
        assert_eq!(tr("Vertex"), "Вершина");
        preferences::edit(|p| p.language = Language::Polish);
        assert_eq!(tr("Vertex"), "Wierzchołek");
        preferences::edit(|p| p.language = Language::Italian);
        assert_eq!(tr("Vertex"), "Vertice");
        preferences::edit(|p| p.language = Language::English);
        assert_eq!(tr("Vertex"), "Vertex");
    }
}
