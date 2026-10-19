//! Heron Brush's desktop executable. The application itself is the `heron_brush`
//! library, which the Android build shares.

// A release build on Windows is a GUI app: no console window opens beside it. Piped
// stdio (`--mcp`, `--version` read by a script) still works.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    heron_brush::run_desktop()
}

/// The web build starts from crates/heron-brush-web, not from here.
#[cfg(target_arch = "wasm32")]
fn main() {}
