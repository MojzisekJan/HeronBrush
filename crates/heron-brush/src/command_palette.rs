//! The command palette: every menu command in one list under a search field, opened with
//! F3 or Ctrl+Shift+P, so a command can be found by name and run from the keyboard.

use eframe::egui;

use crate::menu::Entry;
use crate::theme;
use crate::widgets;
use editor_core::i18n::tr;

/// How many matches the list shows at once.
const SHOWN: usize = 12;

/// The palette's state while it is open.
#[derive(Default)]
pub struct Palette {
    /// Whether it is showing.
    pub open: bool,
    /// What has been typed.
    query: String,
    /// Which match is lit, by its place in the list.
    lit: usize,
    /// Whether the field still has to be given the keyboard.
    focus: bool,
}

/// One command the palette offers: where it is in the menus, its name, its shortcut and
/// what it does.
struct Command<A> {
    path: String,
    label: String,
    shortcut: String,
    action: A,
}

/// The enabled items of `entries`, submenus included, each with the menus above it.
fn flatten<A: Clone>(path: &str, entries: &[Entry<A>], out: &mut Vec<Command<A>>) {
    for entry in entries {
        match entry {
            Entry::Item {
                label,
                shortcut,
                action,
                enabled: true,
                ..
            } => out.push(Command {
                path: path.to_string(),
                label: label.clone(),
                shortcut: shortcut.clone(),
                action: action.clone(),
            }),
            Entry::Sub { label, entries } => {
                flatten(&format!("{path} › {label}"), entries, out);
            }
            Entry::Item { .. } | Entry::Separator => {}
        }
    }
}

/// Whether `command` matches `query`: every word of the query is in its name or its
/// menu's, whatever the case.
fn matches<A>(command: &Command<A>, query: &str) -> bool {
    let haystack = format!("{} {}", command.label, command.path).to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

impl Palette {
    /// Whether this frame's keys ask for the palette: F3, or Ctrl+Shift+P.
    pub fn requested(ctx: &egui::Context) -> bool {
        ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::F3)
                || i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::P,
                )
        })
    }

    /// Opens the palette empty, with the keyboard in its field.
    pub fn open(&mut self) {
        *self = Self {
            open: true,
            focus: true,
            ..Self::default()
        };
    }

    /// Shows the palette over the top of `screen` while it is open, offering the commands
    /// of `menus`, named by `titles`. Returns the command picked; picking one, Escape or a
    /// click elsewhere closes it.
    pub fn show<A: Clone>(
        &mut self,
        ctx: &egui::Context,
        screen: egui::Rect,
        titles: &[&'static str],
        menus: &[Vec<Entry<A>>],
    ) -> Option<A> {
        if !self.open {
            return None;
        }
        let mut commands = Vec::new();
        for (title, entries) in titles.iter().zip(menus) {
            flatten(tr(title), entries, &mut commands);
        }
        let found: Vec<&Command<A>> = commands
            .iter()
            .filter(|command| matches(command, &self.query))
            .collect();
        let none = egui::Modifiers::NONE;
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(none, egui::Key::ArrowUp),
                i.consume_key(none, egui::Key::ArrowDown),
                i.consume_key(none, egui::Key::Enter),
                i.consume_key(none, egui::Key::Escape),
            )
        });
        let count = found.len().min(SHOWN);
        if count > 0 {
            if down {
                self.lit = (self.lit + 1) % count;
            }
            if up {
                self.lit = (self.lit + count - 1) % count;
            }
            self.lit = self.lit.min(count - 1);
        }
        let mut picked = enter
            .then(|| found.get(self.lit))
            .flatten()
            .map(|c| c.action.clone());
        let width = 520.0_f32.min(screen.width() - 32.0);
        let at = egui::pos2(screen.center().x - width / 2.0, screen.top() + 64.0);
        let area = egui::Area::new(egui::Id::unique("command-palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                crate::menu::frame(ui.style()).show(ui, |ui| {
                    ui.set_width(width);
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .id(egui::Id::unique("command-palette-field"))
                            .hint_text(tr("Search commands"))
                            .desired_width(f32::INFINITY)
                            .margin(widgets::body_entry_margin(ui, 8, 8))
                            .min_size(egui::vec2(0.0, widgets::row_height(ui))),
                    );
                    if std::mem::take(&mut self.focus) {
                        field.request_focus();
                    }
                    if field.changed() {
                        self.lit = 0;
                    }
                    ui.add_space(4.0);
                    if found.is_empty() {
                        ui.label(
                            egui::RichText::new(tr("No matching commands")).color(theme::dim()),
                        );
                    }
                    for (index, command) in found.iter().take(SHOWN).enumerate() {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(width, widgets::row_height(ui) + 4.0),
                            egui::Sense::click(),
                        );
                        let lit = index == self.lit || response.hovered();
                        let painter = ui.painter();
                        if lit {
                            painter.rect_filled(
                                rect,
                                egui::CornerRadius::same(4),
                                theme::button_hover(),
                            );
                        }
                        theme::text(
                            painter,
                            egui::pos2(rect.left() + 8.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &command.label,
                            theme::body(),
                            theme::fg(),
                        );
                        // The menu it is in, then its shortcut, at the row's end.
                        let end = format!("{}   {}", command.path, command.shortcut);
                        theme::text(
                            painter,
                            egui::pos2(rect.right() - 8.0, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            end.trim_end(),
                            theme::body(),
                            theme::dim(),
                        );
                        if response.clicked() {
                            picked = Some(command.action.clone());
                        }
                    }
                });
            });
        let outside = ctx.input(|i| i.pointer.any_pressed())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| !area.response.rect.contains(p));
        if picked.is_some() || escape || outside {
            self.open = false;
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(path: &str, label: &str) -> Command<u8> {
        Command {
            path: path.into(),
            label: label.into(),
            shortcut: String::new(),
            action: 0,
        }
    }

    #[test]
    fn every_word_of_the_query_has_to_match_name_or_menu() {
        let save = command("File", "Save as");
        assert!(matches(&save, ""));
        assert!(matches(&save, "save"));
        assert!(matches(&save, "file AS"));
        assert!(!matches(&save, "save open"));
    }

    #[test]
    fn submenus_are_flattened_and_disabled_items_left_out() {
        let entries = vec![
            Entry::item("Undo", 1u8),
            Entry::item("Redo", 2u8).enabled(false),
            Entry::Separator,
            Entry::Sub {
                label: "Grid".into(),
                entries: vec![Entry::item("Snap", 3u8)],
            },
        ];
        let mut out = Vec::new();
        flatten("Edit", &entries, &mut out);
        let names: Vec<(&str, &str)> = out
            .iter()
            .map(|c| (c.path.as_str(), c.label.as_str()))
            .collect();
        assert_eq!(names, [("Edit", "Undo"), ("Edit › Grid", "Snap")]);
    }
}
