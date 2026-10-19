# Linux release installation

The version is defined once in `Cargo.toml` under `[workspace.package]` and inherited
by every workspace crate. `heron-brush --version`, Welcome, About and MCP report it.
The first numbered development release is `0.1.0-alpha.1`. Increment the alpha
number for the next preview; use a stable version only once that release is ready.
Update workspace package entries in `Cargo.lock` when changing the version.

Build and install for the current user:

```sh
CARGO_BUILD_JOBS=2 cargo build --release -p heron-brush --locked --offline
python3 scripts/install_linux.py --default-map-handler
```

The installation contains the executable (copied as `heron-brush`), asset maps and
previews, materials, normal maps, bundled plugin sources and license. It uses the
host's OpenGL driver and windowing libraries (Wayland or X11); this is a local Linux
installation, not a cross-distribution bundle.
The default Cargo features include LAN collaboration. Internet collaboration through
iroh requires a separate build with that feature enabled.

- Releases: `~/.local/opt/heron-brush/releases/<version>+<timestamp>.<commit>[.dirty]`
- Active release: `~/.local/opt/heron-brush/current`
- Terminal command: `~/.local/bin/heron-brush`
- Desktop launcher: `~/.local/share/applications/io.github.mojzisekjan.HeronBrush.desktop`

The desktop identity (`io.github.mojzisekjan.HeronBrush`) stays stable across releases; the window reports it as its app ID.
The launcher uses the active release and registers `text/x-quakemap`. With
`--default-map-handler`, double-clicking a map opens it in Heron Brush. Without that
option, the editor is only added to the available applications for maps.

Each installation preserves previous releases. `BUILD.json` records the source
commit, whether the working tree contained changes, and SHA-256 checksums of shipped
files. A dirty build includes the current local work and is not an official tagged
Git release. Installing does not commit, tag or publish anything.

To switch to a previous release, point `current` at its directory:

```sh
ln -sfn "$HOME/.local/opt/heron-brush/releases/EXISTING_RELEASE" "$HOME/.local/opt/heron-brush/current"
```

Restart the editor after switching. Preferences and user plugins stay in their
existing user configuration directories. If `~/.local/bin` is not already on PATH,
add it in the shell's login profile. The installer does not overwrite shell settings.

## Relocatable archive and release gate

Package an existing installation after its smoke checks:

```sh
python3 scripts/package_linux.py --release "$HOME/.local/opt/heron-brush/current" --output /tmp/HeronBrush-linux-x86_64.tar.gz
python3 scripts/verify_linux_package.py /tmp/HeronBrush-linux-x86_64.tar.gz
```

The packager checks every installed file against BUILD.json, rejects links and
unlisted files, and preserves the installed executable's source commit. It does
not assign the current checkout revision to an older installed binary. The archive
includes assets, games, plugins, BUILD.json, a direct-dependency audit and a separate
PACKAGE.json with file checksums. Its sibling .sha256 verifies the archive itself.
The executable runs without installation when assets/ and games/ remain beside it.

The verifier extracts into a fresh temporary directory, checks all package hashes,
isolates XDG locations, and runs MCP from an unrelated empty working directory. It
checks plugin discovery and opens/exports a bundled reference map to Three.js.
This proves relocation on the current host, not another distribution or GPU.

On 15 October the installed water release e2663ce passed this flow with 348 files
and airliner.map. That archive predates the subsequent Unity PBR exporter change.
Its required GLIBC symbol version is 2.39. A final candidate must be rebuilt and
repackaged after the remaining release changes.

The manual Linux release candidate workflow builds on ubuntu-22.04 and rejects
binaries requiring GLIBC newer than 2.35. It packages and verifies the artifact,
then uploads it as a workflow artifact without publishing a GitHub release. This
workflow has not been run remotely yet. Clean-machine GUI/GPU checks remain open.


## Local candidate verified on 16 October 2026

A clean-source release build from `ec4446c2108322df9a874281eb7707efef55c7f9`
was staged under `/tmp/heron-linux-candidate-20261016-z11s33ie/`, with separate
config/data/cache/bin directories. This did not replace the user's installed
application or preferences. The resulting archive is
`/tmp/heron-linux-candidate-20261016-z11s33ie/HeronBrush-linux-x86_64-ec4446c.tar.gz`.
It contains 103 bundled maps, current profiles/plugins and the recent material,
PCX/WAL, 2D and archive changes. BUILD.json records a clean working tree.

SHA-256: `993b31a32be0fc653b859d73757eb8ac34f0a2fb7d61d5a3b68227af0cada0e4`.
The sibling `.sha256` file is included locally. The verifier passed 348 file
checks, isolated-XDG plugin discovery, opening airliner.map and exporting its
geometry to Three.js from an unrelated working directory. Two package regression
tests and twelve web bridge tests also passed. The release build used locked,
offline dependencies and two Cargo build jobs. No tools were installed or fetched.

The dependency audit still reports GLIBC_2.39. Older-baseline builds and clean-host
GUI/GPU checks remain unverified. Nothing was published. The user's active
installation remains the earlier 39aeed25 build. Logs:
`/tmp/heron-linux-candidate-current-build.log`,
`/tmp/heron-linux-candidate-current-stage.log`,
`/tmp/heron-linux-candidate-current-package.log`,
`/tmp/heron-linux-candidate-current-verify.log`,
`/tmp/heron-current-package-tests.log`,
`/tmp/heron-current-web-storage-tests-host.log`.
