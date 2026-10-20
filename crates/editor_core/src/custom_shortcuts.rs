//! User bindings layered over preset commands, with global/map context validation.
use crate::input::{Key, Modifiers};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub key: Key,
    pub modifiers: Modifiers,
}
#[derive(Clone, Copy)]
pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub default: &'static str,
    pub global: bool,
}
pub const COMMANDS: &[Command] = &[
    Command {
        id: "new",
        label: "New map",
        default: "Ctrl+N",
        global: true,
    },
    Command {
        id: "open",
        label: "Open Map",
        default: "Ctrl+O",
        global: true,
    },
    Command {
        id: "save",
        label: "Save",
        default: "Ctrl+S",
        global: true,
    },
    Command {
        id: "save_as",
        label: "Save As",
        default: "Ctrl+Shift+S",
        global: true,
    },
    Command {
        id: "preferences",
        label: "Preferences",
        default: "Ctrl+Comma",
        global: true,
    },
    Command {
        id: "export",
        label: "Export",
        default: "",
        global: true,
    },
    Command {
        id: "build",
        label: "Build and run",
        default: "",
        global: true,
    },
    Command {
        id: "preview",
        label: "Preview animations",
        default: "",
        global: false,
    },
    Command {
        id: "undo",
        label: "Undo",
        default: "Ctrl+Z",
        global: false,
    },
    Command {
        id: "redo",
        label: "Redo",
        default: "Ctrl+Shift+Z",
        global: false,
    },
    Command {
        id: "copy",
        label: "Copy",
        default: "Ctrl+C",
        global: false,
    },
    Command {
        id: "cut",
        label: "Cut",
        default: "Ctrl+X",
        global: false,
    },
    Command {
        id: "paste",
        label: "Paste",
        default: "Ctrl+V",
        global: false,
    },
    Command {
        id: "duplicate",
        label: "Duplicate",
        default: "Ctrl+D",
        global: false,
    },
    Command {
        id: "delete",
        label: "Delete",
        default: "Delete",
        global: false,
    },
    Command {
        id: "select_all",
        label: "Select all",
        default: "Ctrl+A",
        global: false,
    },
    Command {
        id: "repeat",
        label: "Repeat last commands",
        default: "Ctrl+R",
        global: false,
    },
    Command {
        id: "tool_select",
        label: "Select",
        default: "Escape",
        global: false,
    },
    Command {
        id: "tool_brush",
        label: "Brush",
        default: "B",
        global: false,
    },
    Command {
        id: "tool_clip",
        label: "Clip",
        default: "C",
        global: false,
    },
    Command {
        id: "tool_vertex",
        label: "Vertex",
        default: "V",
        global: false,
    },
    Command {
        id: "tool_edge",
        label: "Edge",
        default: "E",
        global: false,
    },
    Command {
        id: "tool_face",
        label: "Face",
        default: "F",
        global: false,
    },
    Command {
        id: "tool_rotate",
        label: "Rotate",
        default: "R",
        global: false,
    },
    Command {
        id: "tool_scale",
        label: "Scale",
        default: "T",
        global: false,
    },
    Command {
        id: "tool_shear",
        label: "Shear",
        default: "G",
        global: false,
    },
    Command {
        id: "tool_assemble",
        label: "Assemble",
        default: "N",
        global: false,
    },
    Command {
        id: "tool_sweep",
        label: "Sweep",
        default: "Y",
        global: false,
    },
    Command {
        id: "tool_fillet",
        label: "Fillet",
        default: "O",
        global: false,
    },
    Command {
        id: "tool_paint",
        label: "Paint",
        default: "K",
        global: false,
    },
    Command {
        id: "tool_sculpt",
        label: "Sculpt",
        default: "J",
        global: false,
    },
];
pub type Bindings = BTreeMap<String, String>;
pub fn load(text: &str) -> Result<Bindings, String> {
    if text.is_empty() {
        Ok(Bindings::new())
    } else {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}
pub fn default_chord(command: &Command, preset: crate::preferences::Keymap) -> &'static str {
    if preset == crate::preferences::Keymap::Blender {
        match command.id {
            "tool_select" => "G",
            "tool_brush" | "tool_shear" | "tool_paint" | "tool_sculpt" => "",
            "tool_vertex" => "1",
            "tool_edge" => "2",
            "tool_face" => "3",
            "tool_scale" => "S",
            "tool_sweep" => "E",
            "duplicate" => "Shift+D",
            "select_all" => "A",
            "delete" => "X",
            _ => command.default,
        }
    } else {
        command.default
    }
}
pub fn parse(text: &str) -> Result<Chord, String> {
    let mut modifiers = Modifiers::default();
    let mut key = None;
    for part in text.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmd" | "command" => modifiers.control = true,
            "shift" => modifiers.shift = true,
            "alt" | "option" => modifiers.alt = true,
            raw => {
                if key.is_some() {
                    return Err("Use one key with optional Ctrl/Shift/Alt".into());
                }
                let found = match raw {
                    "esc" | "escape" => Key::Escape,
                    "enter" | "return" => Key::Enter,
                    "tab" => Key::Tab,
                    "space" => Key::Space,
                    "delete" | "del" => Key::Delete,
                    "backspace" => Key::Backspace,
                    "insert" => Key::Insert,
                    "home" => Key::Home,
                    "end" => Key::End,
                    "pageup" => Key::PageUp,
                    "pagedown" => Key::PageDown,
                    "left" => Key::Left,
                    "right" => Key::Right,
                    "up" => Key::Up,
                    "down" => Key::Down,
                    "comma" | "," => Key::Comma,
                    "period" | "." => Key::Period,
                    "slash" | "/" => Key::Slash,
                    "plus" => Key::Plus,
                    "minus" | "-" => Key::Minus,
                    "[" => Key::BracketLeft,
                    "]" => Key::BracketRight,
                    value if value.len() == 1 && value.as_bytes()[0].is_ascii_alphabetic() => {
                        Key::Letter(value.chars().next().unwrap())
                    }
                    value if value.len() == 1 && value.as_bytes()[0].is_ascii_digit() => {
                        Key::Digit(value.as_bytes()[0] - b'0')
                    }
                    value if value.starts_with('f') => {
                        let n = value[1..]
                            .parse::<u8>()
                            .map_err(|_| format!("Unknown key: {part}"))?;
                        if !(1..=24).contains(&n) {
                            return Err("Function keys are F1 through F24".into());
                        }
                        Key::Function(n)
                    }
                    _ => return Err(format!("Unknown key: {part}")),
                };
                key = Some(found);
            }
        }
    }
    Ok(Chord {
        key: key.ok_or("Choose a key")?,
        modifiers,
    })
}
pub fn validate(bindings: &Bindings, preset: crate::preferences::Keymap) -> Result<(), String> {
    for key in bindings.keys() {
        if !COMMANDS.iter().any(|c| c.id == key) {
            return Err(format!("Unknown shortcut command: {key}"));
        }
    }
    let mut used: Vec<(Chord, &str)> = vec![];
    for command in COMMANDS {
        let value = bindings
            .get(command.id)
            .map(String::as_str)
            .unwrap_or_else(|| default_chord(command, preset));
        if value.is_empty() {
            continue;
        }
        let chord = parse(value)?;
        if command.global
            && !chord.modifiers.control
            && !chord.modifiers.alt
            && !matches!(chord.key, Key::Function(_))
        {
            return Err(format!(
                "{}: global shortcuts need Ctrl/Alt or a function key",
                command.label
            ));
        }
        if let Some((_, other)) = used.iter().find(|(used, _)| *used == chord) {
            return Err(format!("{value}: {} conflicts with {other}", command.label));
        }
        used.push((chord, command.label));
    }
    Ok(())
}
pub fn resolve(
    bindings: &Bindings,
    _preset: crate::preferences::Keymap,
    chord: Chord,
    text_field: bool,
) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| {
        (!text_field || command.global)
            && bindings
                .get(command.id)
                .is_some_and(|text| parse(text).ok() == Some(chord))
    })
}
pub fn suppressed(
    bindings: &Bindings,
    preset: crate::preferences::Keymap,
    chord: Chord,
    text_field: bool,
) -> bool {
    let (key, modifiers) = crate::keymap::translate(preset, chord.key, chord.modifiers);
    let canonical = Chord { key, modifiers };
    COMMANDS.iter().any(|command| {
        (!text_field || command.global)
            && !(command.id == "tool_select" && chord.key == Key::Escape)
            && bindings.contains_key(command.id)
            && parse(command.default).ok() == Some(canonical)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::Keymap;
    #[test]
    fn conflicts_and_invalid_global_keys_cannot_be_committed() {
        assert!(validate(&Bindings::new(), Keymap::HeronBrush).is_ok());
        assert!(validate(&Bindings::new(), Keymap::Blender).is_ok());
        let bindings = Bindings::from([("tool_brush".into(), "Ctrl+S".into())]);
        assert!(
            validate(&bindings, Keymap::HeronBrush)
                .unwrap_err()
                .contains("conflicts")
        );
        let bindings = Bindings::from([("save".into(), "S".into())]);
        assert!(validate(&bindings, Keymap::HeronBrush).is_err());
        assert!(parse("F25").is_err());
        assert!(parse("Ctrl+Bogus").is_err());
    }
    #[test]
    fn map_shortcuts_leave_text_fields_alone_and_suppress_old_bindings() {
        let bindings = Bindings::from([
            ("tool_brush".into(), "F5".into()),
            ("save".into(), "F6".into()),
        ]);
        validate(&bindings, Keymap::Blender).unwrap();
        let chord = parse("F5").unwrap();
        assert_eq!(
            resolve(&bindings, Keymap::Blender, chord, false)
                .unwrap()
                .id,
            "tool_brush"
        );
        assert!(resolve(&bindings, Keymap::Blender, chord, true).is_none());
        assert_eq!(
            resolve(&bindings, Keymap::Blender, parse("F6").unwrap(), true)
                .unwrap()
                .id,
            "save"
        );
        assert!(suppressed(
            &bindings,
            Keymap::Blender,
            parse("Ctrl+S").unwrap(),
            false
        ));
        let text = serde_json::to_string(&bindings).unwrap();
        assert_eq!(load(&text).unwrap(), bindings);
    }
}
