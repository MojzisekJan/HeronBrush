//! The Custom shortcuts section of Preferences → Keyboard: every command with its key,
//! a filter, and a button per command that takes the next key pressed. The stored JSON
//! never appears in the UI.
//!
//! The rest of the dialog applies a change as it is made, and so does this section: a
//! set of bindings without conflicts goes into the preferences at once, so the menus
//! show the new keys straight away. A set with a conflict stays a draft here, the
//! conflict is named and its rows marked, until it is resolved.
use editor_core::{
    custom_shortcuts::{self, Bindings, COMMANDS},
    i18n::tr,
    preferences,
};
use eframe::egui;

/// What the section keeps between frames.
#[derive(Clone, Default)]
struct State {
    /// Bindings that do not validate yet, shown instead of the stored ones.
    draft: Option<Bindings>,
    /// The command whose button is waiting for a key.
    capturing: Option<&'static str>,
    filter: String,
}

/// Draws the section across the card's width.
pub fn section(ui: &mut egui::Ui, stored: &mut String) {
    let id = ui.make_persistent_id("shortcut-section");
    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<State>(id))
        .unwrap_or_default();
    let preset = preferences::pref(|p| p.keymap);
    let mut bindings = state
        .draft
        .clone()
        .unwrap_or_else(|| custom_shortcuts::load(stored).unwrap_or_default());
    let mut changed = false;

    ui.label(tr(
        "Global commands also work in text fields. Map commands leave text entry alone.",
    ));
    ui.label(tr(
        "Example: Ctrl+Shift+B or F5. Clear a binding to disable it; Reset restores the preset.",
    ));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let width = ui.available_width() - 160.0;
        ui.scope(|ui| {
            ui.set_width(width.max(120.0));
            crate::widgets::search(ui, &mut state.filter, tr("Search by name or description"));
        });
        if ui
            .add_enabled(
                !bindings.is_empty(),
                egui::Button::new(tr("Restore defaults")),
            )
            .clicked()
        {
            bindings.clear();
            changed = true;
        }
    });

    // A key pressed while a button waits is the binding, and nothing else sees it: not
    // the dialog's Escape, nor Tab moving the focus.
    if let Some(command) = state.capturing {
        if let Some(chord) = ui.input(|i| i.events.iter().find_map(captured_chord)) {
            bindings.insert(command.into(), chord);
            state.capturing = None;
            changed = true;
        }
    }

    let conflict = custom_shortcuts::validate(&bindings, preset).err();
    let needle = state.filter.trim().to_lowercase();
    ui.add_space(4.0);
    for command in COMMANDS {
        let chord = bindings
            .get(command.id)
            .cloned()
            .unwrap_or_else(|| custom_shortcuts::default_chord(command, preset).into());
        if !needle.is_empty()
            && !tr(command.label).to_lowercase().contains(&needle)
            && !command.label.to_lowercase().contains(&needle)
            && !chord.to_lowercase().contains(&needle)
        {
            continue;
        }
        let clashes = conflict
            .as_deref()
            .is_some_and(|error| !chord.is_empty() && error.starts_with(&format!("{chord}:")));
        ui.push_id(command.id, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::column_label(ui, tr(command.label), 155.0);
                crate::widgets::column_label(
                    ui,
                    tr(if command.global { "Global" } else { "Map" }),
                    60.0,
                );
                let waiting = state.capturing == Some(command.id);
                let text = if waiting {
                    "…".to_string()
                } else if chord.is_empty() {
                    "—".to_string()
                } else {
                    chord.clone()
                };
                let mut label = egui::RichText::new(text);
                if clashes {
                    label = label.color(crate::theme::error());
                }
                let button = ui.add_sized(
                    egui::vec2(140.0, crate::widgets::ROW_HEIGHT),
                    egui::Button::new(label).selected(waiting),
                );
                if button.clicked() {
                    state.capturing = if waiting { None } else { Some(command.id) };
                    button.request_focus();
                }
                if waiting {
                    // Keep the keyboard here, Tab and arrows included, until a key comes.
                    ui.memory_mut(|memory| {
                        memory.request_focus(button.id);
                        memory.set_focus_lock_filter(
                            button.id,
                            egui::EventFilter {
                                tab: true,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: true,
                            },
                        );
                    });
                }
                if ui
                    .add_enabled(!chord.is_empty(), egui::Button::new(tr("Clear")))
                    .clicked()
                {
                    bindings.insert(command.id.into(), String::new());
                    changed = true;
                }
                if ui
                    .add_enabled(
                        bindings.contains_key(command.id),
                        egui::Button::new(tr("Reset")),
                    )
                    .clicked()
                {
                    bindings.remove(command.id);
                    changed = true;
                }
            });
        });
    }
    if let Some(error) = &conflict {
        ui.colored_label(crate::theme::error(), error);
    }

    if changed {
        if custom_shortcuts::validate(&bindings, preset).is_ok() {
            *stored = serde_json::to_string(&bindings).expect("bindings serialize");
            state.draft = None;
        } else {
            state.draft = Some(bindings);
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, state));
}

/// The chord a key event names, as the bindings write it, if it is one they can hold.
/// egui turns Ctrl+C, Ctrl+X and Ctrl+V into clipboard events, so those count too.
fn captured_chord(event: &egui::Event) -> Option<String> {
    let (key, modifiers) = match event {
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => (key.name().to_string(), *modifiers),
        egui::Event::Copy => ("C".into(), egui::Modifiers::COMMAND),
        egui::Event::Cut => ("X".into(), egui::Modifiers::COMMAND),
        egui::Event::Paste(_) => ("V".into(), egui::Modifiers::COMMAND),
        _ => return None,
    };
    let mut chord = String::new();
    if modifiers.command || modifiers.ctrl {
        chord.push_str("Ctrl+");
    }
    if modifiers.alt {
        chord.push_str("Alt+");
    }
    if modifiers.shift {
        chord.push_str("Shift+");
    }
    chord.push_str(&key);
    custom_shortcuts::parse(&chord).ok().map(|_| chord)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn captured_keys_are_written_as_the_bindings_read_them() {
        let ctrl_shift = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
        assert_eq!(
            captured_chord(&key(egui::Key::B, ctrl_shift)).as_deref(),
            Some("Ctrl+Shift+B")
        );
        assert_eq!(
            captured_chord(&key(egui::Key::F5, egui::Modifiers::NONE)).as_deref(),
            Some("F5")
        );
        assert_eq!(
            captured_chord(&key(egui::Key::Escape, egui::Modifiers::NONE)).as_deref(),
            Some("Escape")
        );
        assert_eq!(
            captured_chord(&egui::Event::Copy).as_deref(),
            Some("Ctrl+C")
        );
        for chord in ["Ctrl+Shift+B", "F5", "Escape", "Ctrl+C"] {
            assert!(custom_shortcuts::parse(chord).is_ok());
        }
        // A key the bindings cannot hold is not taken.
        assert!(captured_chord(&key(egui::Key::Pipe, egui::Modifiers::NONE)).is_none());
    }
}
