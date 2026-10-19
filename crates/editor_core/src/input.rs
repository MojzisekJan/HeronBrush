//! Input as the core reads it, whatever toolkit it came from: the modifier keys held and
//! the keys the editor's own shortcuts and view controls answer to.

/// Which modifier keys are held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift.
    pub shift: bool,
    /// Control (Command on macOS, where the frontend maps it).
    pub control: bool,
    /// Alt (Option on macOS).
    pub alt: bool,
}

/// A key, by what it means rather than by keycode. Letters are case-insensitive; the
/// frontend reports Shift in [`Modifiers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A letter, lower case.
    Letter(char),
    /// A digit on the main row, 0 to 9.
    Digit(u8),
    /// A digit on the numeric keypad, 0 to 9.
    Keypad(u8),
    /// A function key, F1 to F24.
    Function(u8),
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    /// `+` or `=`, the key the grid grows with.
    Plus,
    /// `-`.
    Minus,
    /// `[`.
    BracketLeft,
    /// `]`.
    BracketRight,
    /// `,`.
    Comma,
    /// `.`.
    Period,
    /// `/`.
    Slash,
    /// `#`, which toggles the grid on layouts that have it.
    NumberSign,
    /// Anything else.
    Other,
}

impl Key {
    /// Whether this is the letter `c`, in either case.
    pub fn is(self, c: char) -> bool {
        self == Key::Letter(c.to_ascii_lowercase())
    }
}

/// Which digit a key stands for: the digit it types, or, when it types something else --
/// the Czech, German and French number rows type letters -- the digit printed on the
/// physical key, which the frontend reads from the hardware.
pub fn digit(key: Key, hardware_digit: Option<u32>) -> Option<u32> {
    match key {
        Key::Digit(n) | Key::Keypad(n) => Some(u32::from(n)),
        _ => hardware_digit,
    }
}
