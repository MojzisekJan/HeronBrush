//! The built-in user manual, `docs/manual/<code>.md`, compiled into the binary so it
//! works offline, and the chapters both frontends show it in.
//!
//! English is the original. A translation is registered by one line in [`MANUALS`]:
//!
//! ```text
//! Manual { language: Language::German, text: include_str!("../../../docs/manual/de.md"), search: "Dokumentation durchsuchen", nothing: "Keine passenden Kapitel" },
//! ```
//!
//! The file names follow the UI language codes: `cs`, `de`, `zh`, `es`, `uk`, `pl`, `it`,
//! `fr`, `pt` (Brazilian Portuguese), `ja`, `ko`, `tr` and `id`; a new one takes its UI
//! language code from `i18n::LANGUAGES` (`ru`, `nl`, ...). A language without a file
//! gets the English manual. The test below checks that every translation has the same
//! chapters and sections as English and keeps its shortcuts and command-line flags;
//! `docs/manual/TRANSLATING.md` says the rest.

use crate::i18n::Language;

/// One translation of the manual.
pub struct Manual {
    /// The UI language it is for.
    pub language: Language,
    /// The Markdown source: `# ` starts a chapter, `## ` a section.
    pub text: &'static str,
    /// The search field's placeholder, in this language.
    pub search: &'static str,
    /// What the reader says when no chapter matches the search, in this language.
    pub nothing: &'static str,
}

/// Every manual there is, English first. Add a translation here, one line each.
pub const MANUALS: &[Manual] = &[
    Manual {
        language: Language::English,
        text: include_str!("../../../docs/manual/en.md"),
        search: "Search documentation",
        nothing: "No matching chapters",
    },
    Manual {
        language: Language::Czech,
        text: include_str!("../../../docs/manual/cs.md"),
        search: "Hledat v dokumentaci",
        nothing: "Žádná odpovídající kapitola",
    },
    Manual {
        language: Language::German,
        text: include_str!("../../../docs/manual/de.md"),
        search: "Dokumentation durchsuchen",
        nothing: "Keine passenden Kapitel",
    },
    Manual {
        language: Language::Chinese,
        text: include_str!("../../../docs/manual/zh.md"),
        search: "搜索文档",
        nothing: "没有匹配的章节",
    },
    Manual {
        language: Language::Spanish,
        text: include_str!("../../../docs/manual/es.md"),
        search: "Buscar en la documentación",
        nothing: "Ningún capítulo coincide",
    },
    Manual {
        language: Language::Ukrainian,
        text: include_str!("../../../docs/manual/uk.md"),
        search: "Пошук у документації",
        nothing: "Немає відповідних розділів",
    },
    Manual {
        language: Language::Polish,
        text: include_str!("../../../docs/manual/pl.md"),
        search: "Szukaj w dokumentacji",
        nothing: "Brak pasujących rozdziałów",
    },
    Manual {
        language: Language::Italian,
        text: include_str!("../../../docs/manual/it.md"),
        search: "Cerca nella documentazione",
        nothing: "Nessun capitolo corrispondente",
    },
    Manual {
        language: Language::French,
        text: include_str!("../../../docs/manual/fr.md"),
        search: "Rechercher dans la documentation",
        nothing: "Aucun chapitre correspondant",
    },
    Manual {
        language: Language::PortugueseBrazil,
        text: include_str!("../../../docs/manual/pt.md"),
        search: "Pesquisar na documentação",
        nothing: "Nenhum capítulo correspondente",
    },
    Manual {
        language: Language::Japanese,
        text: include_str!("../../../docs/manual/ja.md"),
        search: "ドキュメントを検索",
        nothing: "一致する章がありません",
    },
    Manual {
        language: Language::Korean,
        text: include_str!("../../../docs/manual/ko.md"),
        search: "문서 검색",
        nothing: "일치하는 장 없음",
    },
    Manual {
        language: Language::Turkish,
        text: include_str!("../../../docs/manual/tr.md"),
        search: "Belgelerde ara",
        nothing: "Eşleşen bölüm yok",
    },
    Manual {
        language: Language::Indonesian,
        text: include_str!("../../../docs/manual/id.md"),
        search: "Cari dokumentasi",
        nothing: "Tidak ada bab yang cocok",
    },
    Manual {
        language: Language::Russian,
        text: include_str!("../../../docs/manual/ru.md"),
        search: "Поиск в документации",
        nothing: "Нет подходящих глав",
    },
    Manual {
        language: Language::Bulgarian,
        text: include_str!("../../../docs/manual/bg.md"),
        search: "Търсене в документацията",
        nothing: "Няма съвпадащи глави",
    },
    Manual {
        language: Language::Slovak,
        text: include_str!("../../../docs/manual/sk.md"),
        search: "Hľadať v dokumentácii",
        nothing: "Žiadne zodpovedajúce kapitoly",
    },
    Manual {
        language: Language::Slovenian,
        text: include_str!("../../../docs/manual/sl.md"),
        search: "Iskanje po dokumentaciji",
        nothing: "Ni ustreznih poglavij",
    },
    Manual {
        language: Language::Dutch,
        text: include_str!("../../../docs/manual/nl.md"),
        search: "Documentatie doorzoeken",
        nothing: "Geen overeenkomende hoofdstukken",
    },
    Manual {
        language: Language::Swedish,
        text: include_str!("../../../docs/manual/sv.md"),
        search: "Sök i dokumentationen",
        nothing: "Inga matchande kapitel",
    },
    Manual {
        language: Language::Danish,
        text: include_str!("../../../docs/manual/da.md"),
        search: "Søg i dokumentationen",
        nothing: "Ingen matchende kapitler",
    },
    Manual {
        language: Language::NorwegianBokmal,
        text: include_str!("../../../docs/manual/nb.md"),
        search: "Søk i dokumentasjonen",
        nothing: "Ingen samsvarende kapitler",
    },
    Manual {
        language: Language::Finnish,
        text: include_str!("../../../docs/manual/fi.md"),
        search: "Hae dokumentaatiosta",
        nothing: "Ei vastaavia lukuja",
    },
    Manual {
        language: Language::Hungarian,
        text: include_str!("../../../docs/manual/hu.md"),
        search: "Keresés a dokumentációban",
        nothing: "Nincs egyező fejezet",
    },
    Manual {
        language: Language::Romanian,
        text: include_str!("../../../docs/manual/ro.md"),
        search: "Caută în documentație",
        nothing: "Niciun capitol potrivit",
    },
    Manual {
        language: Language::Lithuanian,
        text: include_str!("../../../docs/manual/lt.md"),
        search: "Ieškoti dokumentacijoje",
        nothing: "Atitinkančių skyrių nėra",
    },
    Manual {
        language: Language::Greek,
        text: include_str!("../../../docs/manual/el.md"),
        search: "Αναζήτηση στην τεκμηρίωση",
        nothing: "Κανένα κεφάλαιο δεν ταιριάζει",
    },
    Manual {
        language: Language::Vietnamese,
        text: include_str!("../../../docs/manual/vi.md"),
        search: "Tìm trong tài liệu",
        nothing: "Không có chương phù hợp",
    },
    Manual {
        language: Language::Croatian,
        text: include_str!("../../../docs/manual/hr.md"),
        search: "Pretraži dokumentaciju",
        nothing: "Nema odgovarajućih poglavlja",
    },
    Manual {
        language: Language::ChineseTraditional,
        text: include_str!("../../../docs/manual/zh-TW.md"),
        search: "搜尋說明文件",
        nothing: "沒有符合的章節",
    },
];

/// The manual for `language` (resolving "follow the desktop"), or the English one when
/// that language has none.
pub fn manual(language: Language) -> &'static Manual {
    let language = language.resolved();
    MANUALS
        .iter()
        .find(|m| m.language == language)
        .unwrap_or(&MANUALS[0])
}

/// The manual's Markdown in `language`, or in English when that language has none.
pub fn source(language: Language) -> &'static str {
    manual(language).text
}

/// The languages the manual can be read in, in the order the reader's chooser lists them.
pub fn languages() -> impl Iterator<Item = Language> {
    MANUALS.iter().map(|m| m.language)
}

/// One `# ` chapter: its title and the lines under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub title: String,
    pub body: String,
}

/// The manual in `language`, split into chapters.
pub fn chapters(language: Language) -> Vec<Chapter> {
    let mut chapters: Vec<Chapter> = Vec::new();
    for line in source(language).lines() {
        if let Some(title) = line.strip_prefix("# ") {
            chapters.push(Chapter {
                title: title.to_owned(),
                body: String::new(),
            });
        } else if let Some(chapter) = chapters.last_mut() {
            chapter.body.push_str(line);
            chapter.body.push('\n');
        }
    }
    chapters
}

/// Words a reader may use for the same thing. A query word close to one of a group's
/// words also finds the others: "copy texture" finds "paste its material".
const SYNONYMS: &[&[&str]] = &[
    &["copy", "paste", "apply", "transfer", "clone"],
    &["texture", "material", "surface"],
    &["face", "side", "plane"],
];

/// Whether a chapter's title or text matches `query`, ignoring case: the whole query as
/// written, or each of its words, in any order, within the title or within one paragraph.
/// A word matches part of a word (`extru` finds "extrude"), a word one or two typing
/// slips away (`textre` finds "texture"), and the other words of its [`SYNONYMS`] group.
pub fn matches(chapter: &Chapter, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    let title = chapter.title.to_lowercase();
    let body = chapter.body.to_lowercase();
    if title.contains(&query) || body.contains(&query) {
        return true;
    }
    let terms: Vec<Vec<String>> = query.split_whitespace().map(alternatives).collect();
    if terms.is_empty() {
        return false;
    }
    std::iter::once(title.as_str())
        .chain(body.lines())
        .any(|paragraph| {
            let words: Vec<&str> = paragraph
                .split(|c: char| !c.is_alphanumeric() && c != '+')
                .filter(|word| !word.is_empty())
                .collect();
            terms.iter().all(|alternatives| {
                alternatives
                    .iter()
                    .any(|term| words.iter().any(|word| word_matches(term, word)))
            })
        })
}

/// A query word with the words of every synonym group it is close to.
fn alternatives(word: &str) -> Vec<String> {
    let mut out = vec![word.to_string()];
    for group in SYNONYMS {
        if group.iter().any(|synonym| word_matches(word, synonym)) {
            out.extend(group.iter().map(|synonym| synonym.to_string()));
        }
    }
    out
}

/// Whether a text word holds the query word, or nearly does.
fn word_matches(term: &str, word: &str) -> bool {
    if word.contains(term) {
        return true;
    }
    let length = term.chars().count();
    let slips = match length {
        0..=3 => return false,
        4..=7 => 1,
        _ => 2,
    };
    // The whole word, or its start as long as the term, so a slip in a partial word
    // still counts.
    let start: String = word.chars().take(length).collect();
    edit_distance(term, word) <= slips || edit_distance(term, &start) <= slips
}

/// How many single-character insertions, deletions or substitutions turn `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many lines start with `prefix`, which ends in a space so `## ` does not count `### `.
    fn count(text: &str, prefix: &str) -> usize {
        text.lines().filter(|line| line.starts_with(prefix)).count()
    }

    /// The `` `code` `` spans of a text.
    fn code_spans(text: &str) -> Vec<&str> {
        text.lines()
            .flat_map(|line| line.split('`').skip(1).step_by(2))
            .filter(|span| !span.is_empty())
            .collect()
    }

    /// A span a reader types or presses exactly as written, in any language.
    fn is_key_or_flag(span: &str) -> bool {
        span.starts_with("--")
            || ["Ctrl", "Cmd", "Shift"]
                .iter()
                .any(|key| span.contains(key))
    }

    #[test]
    fn every_manual_translation_matches_the_english_structure() {
        let english = MANUALS[0].text;
        assert_eq!(MANUALS[0].language, Language::English);
        let chapters = count(english, "# ");
        let sections = count(english, "## ");
        assert!(chapters > 10 && sections > chapters);
        let keys: Vec<&str> = code_spans(english)
            .into_iter()
            .filter(|span| is_key_or_flag(span))
            .collect();
        assert!(
            keys.contains(&"Ctrl+Z") && keys.contains(&"--mcp"),
            "the shortcuts are written as code"
        );
        for manual in &MANUALS[1..] {
            let name = manual.language.name();
            assert_eq!(
                count(manual.text, "# "),
                chapters,
                "{name}: the number of chapters"
            );
            assert_eq!(
                count(manual.text, "## "),
                sections,
                "{name}: the number of sections"
            );
            let spans = code_spans(manual.text);
            for key in &keys {
                assert!(spans.contains(key), "{name}: `{key}` is missing");
            }
        }
    }

    #[test]
    fn a_language_without_a_manual_reads_the_english_one() {
        assert_eq!(source(Language::Czech), MANUALS[1].text);
        assert_eq!(source(Language::English), MANUALS[0].text);
        if !languages().any(|l| l == Language::Russian) {
            assert_eq!(source(Language::Russian), MANUALS[0].text);
        }
        let languages: Vec<Language> = languages().collect();
        assert_eq!(languages[0], Language::English);
        for (index, language) in languages.iter().enumerate() {
            assert!(
                !languages[index + 1..].contains(language),
                "one manual per language"
            );
        }
    }

    #[test]
    fn chapters_split_at_top_level_headings_and_search_ignores_case() {
        for language in languages() {
            let chapters = chapters(language);
            assert_eq!(chapters.len(), count(source(language), "# "));
            assert!(chapters.iter().any(|c| matches(c, "ctrl+z")));
            assert!(!chapters
                .iter()
                .any(|c| matches(c, "not-a-manual-phrase-917")));
        }
    }

    #[test]
    fn search_takes_words_in_any_order_partial_words_typos_and_synonyms() {
        let english = chapters(Language::English);
        let found = |query: &str| -> Vec<String> {
            english
                .iter()
                .filter(|c| matches(c, query))
                .map(|c| c.title.clone())
                .collect()
        };
        for query in [
            "copy texture",
            "copy textre",
            "Texture copy",
            "COPY TEXTURE",
            "transfer material",
            "apply texture face",
            "paste textur",
        ] {
            let titles = found(query);
            assert!(
                titles.iter().any(|t| t == "Brushes and modelling tools")
                    && titles.iter().any(|t| t == "Materials and UVs"),
                "{query}: {titles:?}"
            );
        }
        assert!(found("extru")
            .iter()
            .any(|t| t == "Brushes and modelling tools"));
        assert!(found("not-a-manual-phrase-917").is_empty());
        assert!(found("qqqq zzzz").is_empty());
        assert_eq!(edit_distance("textre", "texture"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
    }
}
