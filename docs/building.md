# Building from source

Heron Brush is written in Rust; the window and its controls use egui and eframe, the 3D
view wgpu (or OpenGL). Install a stable Rust toolchain, then from the repository root:

```sh
cargo run -p heron-brush --release -- assets/crate.map
cargo run -p heron-brush --release --no-default-features   # the OpenGL 3D view
cargo run -p heron-brush --features iroh                   # internet collaboration
cargo test --workspace
cargo clippy --workspace --all-targets
```

The optional `iroh` feature enables internet collaboration; collaboration on a local
network works without it.

## Packages

- macOS: `scripts/install-macos.sh` builds an app bundle and installs it to
  `/Applications`.
- Windows: [Windows release builds](windows-release.md).
- Linux: [Linux release](linux-release.md).
- Web: [Web](web.md).
- Android: [Android](android.md).

## The workspace

Each crate under `crates/` has a README of its own: the application (`heron-brush`),
the editor without a user interface (`editor_core`), the map model, the renderers, the
exporters and their helpers.

## Contributing

Use standard Rust collection, iterator, formatting and error-handling APIs. Keep UI
ownership and external-library calls at the application boundaries; geometry and
parsing libraries use safe Rust. Preserve file-format conventions and numerical
invariants when refactoring, and add regression tests for changed behaviour.
