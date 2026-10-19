# Application icons

The source is `crates/heron-brush/assets/logo.svg`. The welcome screen and the title
bar draw it directly; the window icon is the 1024 px PNG generated from it
(`assets/app-icons/heron-brush.png`), set through eframe. The application ID is
`io.github.mojzisekjan.HeronBrush`. Restart the rebuilt app to see changes.

## Linux

A desktop launcher supplies the icon to the application menu and desktop shell.
A bare ELF executable generally retains the file manager's executable icon.

After building, install the launcher for the current user:

```sh
cargo build -p heron-brush --release
python3 scripts/install_desktop.py --binary target/release/heron-brush
```

For development use `--binary target/debug/heron-brush`. The launcher references
that absolute binary path and the checkout as its working directory. Reinstall
it if the checkout moves. It installs into `$XDG_DATA_HOME`, defaulting to
`~/.local/share`. `--data-dir` supports staging the launcher and icon elsewhere.
Raster icon sizes are rendered with `rsvg-convert` when it is installed, otherwise
scaled from the PNG with Pillow; with neither, only the scalable SVG is installed.

## Windows

`crates/heron-brush/build.rs` embeds `assets/app-icons/heron-brush.ico` into the
executable on Windows targets. MSVC builds need the Windows SDK's `rc.exe` on PATH
(for example in a Visual Studio developer shell, as the release workflow sets up);
GNU builds need `windres`. Set `RC` to the resource compiler path if needed,
including for cross-compilation. The icon contains 16, 24, 32, 48, 64, 128 and
256 pixel images.

## macOS

`scripts/install-macos.sh` builds `Heron Brush.app` with
`crates/heron-brush/assets/app-icons/heron-brush.icns` as its bundle icon and
installs it. A bare Mach-O executable is not an app bundle and shows no icon in
Finder.

## Updating the artwork

After changing `logo.svg`, regenerate the platform files and rebuild:

```sh
python3 scripts/generate_app_icons.py
cargo build -p heron-brush
```

The generator requires `rsvg-convert` (librsvg) and Pillow.
The PNG, ICO and ICNS are checked in, so ordinary builds do not require Python.
On Linux, rerun the installer to update the desktop's copy of the SVG.

The display name is Heron Brush; the cargo package and binary are `heron-brush`, and
the macOS, Linux and Windows packages install it under that name. The application ID
is `io.github.mojzisekjan.HeronBrush` and the configuration directory is
`heron-brush` (a profile left by an earlier release is adopted on first start; see
`editor_core::preferences::config_dir`). The Unity Live templates in this repository
use neutral filenames (`live_root.cs`, `live_entity.cs` and `live_window.cs`);
installation gives them the filenames their C# component types require. Standalone
Unity, Flax and Cocos exports use `HeronBrush` script names.
