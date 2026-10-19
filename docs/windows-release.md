# Windows release

The GitHub Actions workflow
[Windows release](https://github.com/MojzisekJan/HeronBrush/actions/workflows/windows-release.yml)
builds a portable package for 64-bit Windows. Choose **Run workflow** in Actions; when
the run succeeds, download the `heron-brush-windows-x64-release` artifact.

Extract the whole ZIP into one folder and run `heron-brush.exe`. The package holds the
executable, the `assets` folder and the licence; it needs no further DLLs, no Rust and
no Visual C++ redistributable. `assets` contains the map assets, textures and, in
`assets/plugins`, the sources of the bundled plugins. On a new profile the editor adds
the `assets` folder beside the executable to its library, so the folder can be carried
on a USB drive.

`BUILD.txt` records the source commit. The build uses the default features, which
include collaboration on a local network; the optional `iroh` feature for internet
collaboration is off. GitHub keeps the artifact for 30 days; it is a build artifact,
not a published release.

The workflow builds on `windows-latest` with the MSVC toolchain
(`cargo build --release --locked -p heron-brush`) and a statically linked C runtime.
`crates/heron-brush/build.rs` embeds the application icon with `rc.exe` from the
Windows SDK. The packaging script copies the binary as `heron-brush.exe`, adds the
assets and the licence, runs `heron-brush.exe --version`, then starts the MCP mode with
a `PATH` holding only the package and `System32`, and checks the answers to
`tools/list` and `list_plugins`.

This check does not cover 3D rendering or the user interface; those are tested by hand
on Windows.
