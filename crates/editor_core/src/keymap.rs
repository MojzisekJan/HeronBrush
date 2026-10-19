//! Shortcut presets: the editor's own bindings, and Blender's translated into them.

use crate::input::{Key, Modifiers};
pub use crate::preferences::Keymap;

/// Turns a key press into the one the editor's own shortcuts are written against.
///
/// A press with no translation is passed through, so anything the table says nothing about
/// keeps working as it always did.
pub fn translate(keymap: Keymap, key: Key, modifiers: Modifiers) -> (Key, Modifiers) {
    match keymap {
        Keymap::HeronBrush => (key, modifiers),
        Keymap::Blender => from_blender(key, modifiers),
    }
}

/// The Blender bindings, as presses in the editor's own set.
///
/// Blender's are modal -- G, R and S start an operator that the next mouse move drives --
/// while these tools stay switched on until another is chosen. The keys are the same; what
/// happens after them is the editor's.
fn from_blender(key: Key, modifiers: Modifiers) -> (Key, Modifiers) {
    let Modifiers { control: ctrl, shift, alt } = modifiers;
    let plain = !ctrl && !alt;

    let none = Modifiers::default();
    let ctrl_only = Modifiers { control: true, ..none };
    let ctrl_shift = Modifiers { control: true, shift: true, ..none };
    let ctrl_alt = Modifiers { control: true, alt: true, ..none };
    let letter = Key::Letter;

    match key {
        // G is grab: moving is what the select tool does here, and Escape is what
        // switches back to it.
        Key::Letter('g') if plain && !shift => (Key::Escape, none),
        // S is scale, which is T here. R for rotate needs no entry: it is R in both.
        Key::Letter('s') if plain && !shift => (letter('t'), none),
        // E extrudes. The nearest thing that is a tool is the sweep, which extrudes the
        // selected faces along a path.
        Key::Letter('e') if plain && !shift => (letter('y'), none),
        // Tab keeps the shared four-mode cycle; 1/2/3 directly select components.
        Key::Digit(1) | Key::Keypad(1) if plain && !shift => (letter('v'), none),
        Key::Digit(2) | Key::Keypad(2) if plain && !shift => (letter('e'), none),
        Key::Digit(3) | Key::Keypad(3) if plain && !shift => (letter('f'), none),
        // K is Blender's knife, and the clip tool is what cuts a brush.
        Key::Letter('k') if plain && !shift => (letter('c'), none),
        // B is box select, which is what a drag on nothing already does, so it goes to
        // the select tool rather than to the edge tool B means here.
        Key::Letter('b') if plain && !shift => (Key::Escape, none),

        // A selects everything and Alt+A drops the selection.
        Key::Letter('a') if plain && !shift => (letter('a'), ctrl_only),
        Key::Letter('a') if alt && !ctrl && !shift => (letter('a'), ctrl_shift),
        // X and Delete both delete. X alone is a menu in Blender; there is nothing to
        // ask here, so it deletes.
        Key::Letter('x') if plain && !shift => (Key::Delete, none),
        // Shift+D duplicates.
        Key::Letter('d') if shift && !ctrl && !alt => (letter('d'), ctrl_only),
        // H hides the selection, Alt+H brings it all back.
        Key::Letter('h') if plain && !shift => (letter('i'), ctrl_alt),
        Key::Letter('h') if alt && !ctrl && !shift => (letter('i'), ctrl_shift),
        // Ctrl+Shift+Z is redo in Blender; here that is Ctrl+Y.
        Key::Letter('z') if ctrl && shift && !alt => (letter('y'), ctrl_only),

        _ => (key, modifiers),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Modifiers = Modifiers { shift: false, control: false, alt: false };
    const CTRL: Modifiers = Modifiers { shift: false, control: true, alt: false };

    #[test]
    fn the_editors_own_keymap_changes_nothing() {
        assert_eq!(translate(Keymap::HeronBrush, Key::Letter('g'), CTRL), (Key::Letter('g'), CTRL));
    }

    #[test]
    fn blenders_select_modes_reach_the_three_handle_tools() {
        assert_eq!(translate(Keymap::Blender, Key::Digit(1), NONE), (Key::Letter('v'), NONE));
        assert_eq!(translate(Keymap::Blender, Key::Digit(2), NONE), (Key::Letter('e'), NONE));
        assert_eq!(translate(Keymap::Blender, Key::Digit(3), NONE), (Key::Letter('f'), NONE));
        assert_eq!(translate(Keymap::Blender, Key::Tab, NONE), (Key::Tab, NONE));
    }

    #[test]
    fn blenders_select_all_and_deselect_all_are_the_editors_own() {
        let alt = Modifiers { alt: true, ..NONE };
        assert_eq!(translate(Keymap::Blender, Key::Letter('a'), NONE), (Key::Letter('a'), CTRL));
        assert_eq!(
            translate(Keymap::Blender, Key::Letter('a'), alt),
            (Key::Letter('a'), Modifiers { control: true, shift: true, alt: false })
        );
    }

    #[test]
    fn a_binding_that_is_the_same_in_both_is_left_alone() {
        // Rotate is R either way, and so is every Ctrl shortcut the two share.
        assert_eq!(translate(Keymap::Blender, Key::Letter('r'), NONE), (Key::Letter('r'), NONE));
        assert_eq!(translate(Keymap::Blender, Key::Letter('z'), CTRL), (Key::Letter('z'), CTRL));
        assert_eq!(translate(Keymap::Blender, Key::Letter('s'), CTRL), (Key::Letter('s'), CTRL));
    }

    #[test]
    fn ctrl_shift_z_redoes_under_blenders_bindings() {
        let ctrl_shift = Modifiers { control: true, shift: true, alt: false };
        assert_eq!(translate(Keymap::Blender, Key::Letter('z'), ctrl_shift), (Key::Letter('y'), CTRL));
    }

    #[test]
    fn saving_is_not_swallowed_by_blenders_scale() {
        // S alone scales, but Ctrl+S must still save: a translation that ignored the
        // modifiers would have eaten it.
        assert_eq!(translate(Keymap::Blender, Key::Letter('s'), CTRL), (Key::Letter('s'), CTRL));
        assert_eq!(translate(Keymap::Blender, Key::Letter('s'), NONE), (Key::Letter('t'), NONE));
    }
}
