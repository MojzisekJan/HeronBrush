# Editor appearance

The editor draws its whole interface with egui, so it looks the same on Linux,
Windows and macOS; no desktop widget theme is loaded. The palette, sizes and the
painted widgets egui has no close equivalent of live in
`crates/heron-brush/src/theme.rs`, with a dark and a light value for every colour.
The interface font is Inter (`crates/heron-brush/fonts/`), with egui's built-in fonts
as fallbacks.

Preferences → View → Colour scheme picks Follow the desktop, Light or Dark. Follow
the desktop takes the system's light/dark setting as eframe reports it. **Panel
opacity (%)** lets the desktop show faintly through the window chrome and panels;
the map views stay opaque. Native file dialogs (`rfd`) and, where the window is not
drawn by the editor itself, the operating system's decorations may still differ
between platforms.

Native Windows/macOS visual comparison remains a manual check.
