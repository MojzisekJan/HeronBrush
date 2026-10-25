//! Offline manual with chapter navigation, search and reader preferences.
use crate::{dialog, form, theme, widgets};
use editor_core::{
    i18n::{fill, tr, Language},
    manual::{self, chapters, matches, Chapter},
    preferences,
};
use eframe::egui;

pub struct ManualPage {
    pub open: bool,
    /// The language the manual is shown in: the UI's, or English when it has none.
    language: Language,
    chapters: Vec<Chapter>,
    query: String,
    selected: usize,
    /// The contents sidebar's width, which its edge drags.
    sidebar: f32,
}
impl Default for ManualPage {
    fn default() -> Self {
        let language = manual::manual(preferences::pref(|p| p.language)).language;
        Self {
            open: false,
            language,
            chapters: chapters(language),
            query: String::new(),
            selected: 0,
            sidebar: 230.0,
        }
    }
}
impl ManualPage {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut close = false;
        self.open = dialog::show(ctx, "manual", tr("Manual"), [1100.0, 760.0], |ui, part| {
            if part == dialog::Part::Actions {
                close = dialog::button(ui, tr("Close"), true).clicked();
                return;
            }
            let mut size = preferences::pref(|p| p.manual_font_size);
            let mut contents = preferences::pref(|p| p.manual_show_contents);
            let mut settings_changed = false;
            let mut search = None;
            ui.horizontal(|ui| {
                ui.set_min_height(widgets::ROW_HEIGHT);
                settings_changed |= widgets::switch(ui, &mut contents)
                    .on_hover_text(tr("Show or hide contents"))
                    .changed();
                ui.label(tr("Contents"));
                widgets::toolbar_gap(ui);
                ui.label(egui::RichText::new(tr("Text size")).color(theme::dim()));
                let mut value = f64::from(size);
                if widgets::spin_width(ui, &mut value, 10.0..=28.0, 1.0, 0, 84.0).changed() {
                    size = value as f32;
                    settings_changed = true;
                }
                widgets::toolbar_gap(ui);
                // Every language with a manual, English first, each named in itself.
                let languages: Vec<Language> = manual::languages().collect();
                let names: Vec<&str> = languages.iter().map(|l| l.name()).collect();
                let mut index = languages
                    .iter()
                    .position(|l| *l == self.language)
                    .unwrap_or(0);
                if widgets::dropdown(ui, "manual-language", &mut index, &names, 150.0) {
                    self.language = languages[index];
                    self.chapters = chapters(self.language);
                }
                // The search takes the rest of the row, on the right.
                widgets::toolbar_gap(ui);
                search = Some(widgets::search(
                    ui,
                    &mut self.query,
                    manual::manual(self.language).search,
                ));
            });
            let search = search.expect("the row draws the search box");
            if settings_changed {
                preferences::edit(|p| {
                    p.manual_font_size = size;
                    p.manual_show_contents = contents;
                });
                if let Err(error) =
                    preferences::current().save(&preferences::Preferences::default_path())
                {
                    editor_core::complain!(
                        "{}",
                        fill("Could not save the preferences: {}", &[&error.to_string()])
                    );
                }
            }
            ui.add_space(6.0);
            // Ctrl+F searches, and shows the contents the matches are listed in.
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
                search.request_focus();
                if !contents {
                    contents = true;
                    preferences::edit(|p| p.manual_show_contents = true);
                    if let Err(error) =
                        preferences::current().save(&preferences::Preferences::default_path())
                    {
                        editor_core::complain!(
                            "{}",
                            fill("Could not save the preferences: {}", &[&error.to_string()])
                        );
                    }
                }
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                if self.query.is_empty() {
                    close = true;
                } else {
                    self.query.clear();
                }
            }
            let shown: Vec<_> = self
                .chapters
                .iter()
                .enumerate()
                .filter(|(_, c)| matches(c, &self.query))
                .map(|(i, _)| i)
                .collect();
            if !shown.contains(&self.selected) {
                self.selected = shown.first().copied().unwrap_or(0);
            }
            if shown.is_empty() {
                ui.label(manual::manual(self.language).nothing);
                return;
            }
            let old = self.selected;
            ui.horizontal_top(|ui| {
                if contents {
                    let sidebar = self.sidebar;
                    egui::ScrollArea::vertical()
                        .id_salt("manual-contents")
                        .max_width(sidebar)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(sidebar);
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let mut rows = Vec::with_capacity(shown.len());
                                for &index in &shown {
                                    let row = form::sidebar_item(
                                        ui,
                                        &self.chapters[index].title,
                                        self.selected == index,
                                        None,
                                        30.0,
                                    );
                                    if row.clicked() {
                                        self.selected = index;
                                    }
                                    rows.push(row);
                                }
                                // Up and Down go through the chapters while one has the
                                // keyboard.
                                if let Some(next) = crate::widgets::list_keys(ui, &rows) {
                                    self.selected = shown[next];
                                }
                            });
                        });
                    // The sidebar's edge, which drags its width.
                    let (edge, response) = ui.allocate_exact_size(
                        egui::vec2(9.0, ui.available_height()),
                        egui::Sense::drag(),
                    );
                    let active = response.hovered() || response.dragged();
                    ui.painter().vline(
                        edge.center().x,
                        edge.y_range(),
                        egui::Stroke::new(
                            1.0,
                            if active {
                                crate::theme::accent()
                            } else {
                                crate::theme::border()
                            },
                        ),
                    );
                    if active {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    if response.dragged() {
                        self.sidebar = (self.sidebar + response.drag_delta().x).clamp(140.0, 520.0);
                    }
                }
                let mut scroll = egui::ScrollArea::vertical().id_salt((
                    "manual-body",
                    self.language.name(),
                    self.selected,
                ));
                if old != self.selected {
                    scroll = scroll.vertical_scroll_offset(0.0);
                }
                scroll.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        let chapter = &self.chapters[self.selected];
                        ui.label(
                            egui::RichText::new(&chapter.title)
                                .size(size * 1.5)
                                .strong(),
                        );
                        ui.add_space(size);
                        let mut code = false;
                        for line in chapter.body.lines() {
                            if line.starts_with("```") {
                                code = !code;
                                continue;
                            }
                            if code {
                                ui.label(egui::RichText::new(line).monospace().size(size));
                            } else if let Some(heading) = line.strip_prefix("### ") {
                                // A step below a section heading, as large as the text.
                                ui.add_space(size * 0.3);
                                ui.label(egui::RichText::new(heading).size(size).strong());
                            } else if let Some(heading) = line.strip_prefix("## ") {
                                ui.add_space(size * 0.5);
                                ui.label(egui::RichText::new(heading).size(size * 1.15).strong());
                            } else if line.is_empty() {
                                ui.add_space(size * 0.5);
                            } else {
                                let text = line
                                    .strip_prefix("- ")
                                    .map(|s| format!("• {s}"))
                                    .unwrap_or_else(|| line.to_owned());
                                ui.label(inline(ui.style(), &text, size));
                            }
                        }
                    });
                });
            });
        }) && !close;
    }
}
/// One paragraph of the manual with its `**strong**` and `` `code` `` spans.
/// The markers are dropped and their spans drawn bold or in the code font.
pub(crate) fn inline(style: &egui::Style, text: &str, size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let plain = egui::TextFormat {
        font_id: egui::FontId::proportional(size),
        color: style.visuals.text_color(),
        ..Default::default()
    };
    for (span, kind) in spans(text) {
        let mut format = plain.clone();
        match kind {
            Span::Strong => format.color = style.visuals.strong_text_color(),
            Span::Code => {
                format.font_id = egui::FontId::monospace(size * 0.95);
                format.background = style.visuals.code_bg_color;
            }
            Span::Plain => {}
        }
        job.append(span, 0.0, format);
    }
    job
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Span {
    Plain,
    Strong,
    Code,
}
/// Splits a line at `**` and backtick pairs. An unpaired marker stays as text.
fn spans(text: &str) -> Vec<(&str, Span)> {
    let mut out = vec![];
    let mut rest = text;
    while !rest.is_empty() {
        let strong = rest.find("**");
        let code = rest.find('`');
        let (start, marker, kind) = match (strong, code) {
            (Some(s), Some(c)) if c < s => (c, "`", Span::Code),
            (Some(s), _) => (s, "**", Span::Strong),
            (None, Some(c)) => (c, "`", Span::Code),
            (None, None) => break,
        };
        let inner = &rest[start + marker.len()..];
        let Some(end) = inner.find(marker).filter(|&end| end > 0) else {
            break;
        };
        if start > 0 {
            out.push((&rest[..start], Span::Plain));
        }
        out.push((&inner[..end], kind));
        rest = &inner[end + marker.len()..];
    }
    if !rest.is_empty() {
        out.push((rest, Span::Plain));
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_markers_become_spans() {
        assert_eq!(
            spans("Click **Apply surface** or see `docs/x.md`."),
            vec![
                ("Click ", Span::Plain),
                ("Apply surface", Span::Strong),
                (" or see ", Span::Plain),
                ("docs/x.md", Span::Code),
                (".", Span::Plain),
            ]
        );
        assert_eq!(
            spans("2 ** 3 and a lone `"),
            vec![("2 ** 3 and a lone `", Span::Plain)]
        );
        assert_eq!(spans("Ctrl+Z"), vec![("Ctrl+Z", Span::Plain)]);
    }
    #[test]
    fn search_finds_body_text_in_both_languages() {
        for language in manual::languages() {
            let chapters = chapters(language);
            assert!(chapters.len() > 8);
            assert!(chapters.iter().any(|c| matches(c, "ctrl+z")));
            assert!(!chapters
                .iter()
                .any(|c| matches(c, "not-a-manual-phrase-917")));
        }
    }
}
