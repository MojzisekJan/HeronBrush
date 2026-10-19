# The web build

Heron Brush runs in the browser as WebAssembly. The page is
`crates/heron-brush-web/index.html` and the module is the `heron-brush-web` crate, a thin
`cdylib` whose start function calls `heron_brush::web::run`. That puts eframe's
`WebRunner` on the page's `<canvas>`. Everything else is the desktop application.

## Building and running

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd crates/heron-brush-web
trunk serve --cargo-profile web-dev  # http://127.0.0.1:8080, rebuilt on every change
trunk build --release --cargo-profile release  # into crates/heron-brush-web/dist; measure this fresh artifact
```

## Hosting

TODO: not published yet. The web build needs more work on quality first; until then
Pages stays off and `.github/workflows/web.yml` runs only when started by hand. Once it
is ready, switch Pages to "GitHub Actions" and let the workflow run on every push to
`main`.

`.github/workflows/web.yml` builds the release with `--public-url ./`, so the page works under a sub-path such as
`https://mojzisekjan.github.io/HeronBrush/`, and publishes `dist` on GitHub Pages. It needs
Pages set to "GitHub Actions" in the repository's settings. The page is static: the
module, `bundle/index.json` and a content-addressed `bundle/startup-<sha256>.bin`,
with `files.txt` for inspecting the packed paths.
The archive requires Python 3 to build, but no server-side code to host.

`.cargo/config.toml` sets `getrandom_backend="wasm_js"` for the wasm target. rhai hashes
through ahash, which seeds itself from getrandom, and getrandom 0.3 wants both that cfg
and its `wasm_js` feature (`crates/editor_core/Cargo.toml`).

## What works and what does not

Where the browser hands out a WebGPU adapter, the window and the 3D view run on wgpu and
render_wgpu draws the 3D view. Elsewhere (Firefox, Linux Chromium without the flag,
older Safari) the page runs eframe on WebGL 2 and the 3D view on the GL renderer
(`crates/render`), whose shaders are rewritten as GLSL ES for GLES 3.0 and WebGL 2
(`for_context`). `?renderer=gl` or `?renderer=wgpu` in the address forces one.
Safari 26's WebGPU loses the prepass depth in a pass that attaches it read-only: the
forward pass then tests the sky and the surfaces against nothing and the 3D view stays
black, with no validation error. In the browser render_wgpu therefore attaches the depth
writable in the forward pass and gives the lighting a copy to sample
(`Targets::depth_copy`); the desktop keeps the read-only attachment.
The GL view draws with a glow context made over the canvas's WebGL 2 context, shared
with the renderer, since in WebGL one glow context cannot use objects another made, and
blits its framebuffer into the canvas from the paint callback
(`crates/heron-brush/src/gl_glue.rs`). The canvas is created with `antialias: false`,
because WebGL refuses a blit into a multisampled canvas. On WebGL the wireframe draws
each triangle's sides as lines (GLES has no polygon line mode) and the shadow map has no
border. The render window previews and renders on WebGL too; the PNG is written through
`file_store`, so it goes to the picked file in Chromium and is downloaded elsewhere.

So far:

- The bundled examples open from the welcome screen. `samples::ensure` hands out their
  paths without writing them, and `Document::open` reads the bundled text
  (`samples::bundled`).
- A page cannot read the system's fonts, so the build carries Noto Sans CJK for
  Chinese, Japanese and Korean in `crates/heron-brush-web/fonts`, cut down to the
  characters of the translations and the manual (`scripts/subset_web_cjk_fonts.py`,
  which needs fonttools; run it again after those change). Characters outside that set,
  typed into names for instance, show as boxes.
- `std::time::Instant` and `SystemTime::now` panic in the browser, so the crates use
  `web_time` instead. File times (`Metadata::modified`) stay `std::time::SystemTime`.
- There are no threads. The thumbnails of maps, prefabs and materials go through
  `crate::pool::Pool`, which on the desktop runs threads and on the web works through
  its queue on the frame thread, about 8 ms a frame, so they fill in a few at a time. A
  prefab's thumbnail is drawn from its brushes there, since the PNGs beside the prefabs
  are not loaded. render_wgpu decodes textures the same way in the browser
  (`textures::Workers::next`); that path needs WebGPU and has not been seen working in a
  browser yet, since headless Chromium gives no picture of a WebGPU canvas. rayon runs
  on the calling thread.
- The editor's own files (maps, preferences, the recent and favourite lists, autosaves)
  go through `editor_core::files`. On the web they live in memory, are loaded from
  IndexedDB before the editor starts and are stored there on every write
  (`crates/heron-brush/src/web.rs`, `web_files.js`).
- File > Open and Save As use the browser's choosers when direct file access is usable.
  The picked file is written in place after the browser grants permission. If those APIs
  are absent or reject the context with SecurityError/NotSupportedError, in-page dialogs
  open a file copy or choose a name for a download. Native cancellation and explicit
  permission refusal remain cancellation. Save fallback does not use a browser prompt.
  A picked map is kept as
  `/heron-brush/picked/<id>/<name>`. Independent picks with the same basename
  get separate entries. Chromium reuses a path when a handle identifies the same
  physical file. Fallback file inputs cannot establish physical identity.
- The data a package ships comes along: after each build Trunk runs
  `scripts/stage-web-bundle.sh`, which packs `assets/` and `gamedata/` into
  `dist/bundle/index.json` and `startup-<sha256>.bin`. The page validates the
  manifest, SHA-256, paths and byte ranges, then
  loads all entries before the editor starts. On 15 October this was 199 entries
  and 8,200,013 payload bytes. A small index and one archive replace 200
  individual requests.
  Prefab root PNG thumbnails and authoring scripts are excluded. The data goes
  under `/heron-brush/install`, where `games/` and `assets/` are found as beside a desktop
  executable. So the heron_brush game, the 103 prefabs, their materials and the plugins are
  there. Games, prefabs, FGDs and materials are read through the `file_store` crate
  (re-exported as `editor_core::files`), which map_model and scene_export use too.
- Folder choosers use the browser's own (a file input with `webkitdirectory`, in every
  major browser). The folder comes in as a copy under `/heron-brush/files/<id>/<name>`, kept
  in IndexedDB, so an asset or material folder added this way is there on the next visit;
  nothing is written back to it. Plugins install the same way (a `.rhai` file or a folder
  with `main.rhai`) and stay installed between visits.
- Export > Write builds the level in memory (`export::export_files`) and downloads it:
  the one file where there is one (a `.glb`), else a zip of the folder the desktop would
  write (`crates/heron-brush/src/zip.rs`). Check builds in `/heron-brush/export`. Writing
  the level on every save is off on the web.
- Live sessions: the browser joins a desktop host. The host's port answers a
  WebSocket handshake as well as the plain line protocol (`collab/net.rs`), and the
  page joins `address:port` over `ws://`. The Live session window starts in Join mode
  there; hosting needs the desktop app, since a page cannot listen for connections, so
  two browsers cannot meet without a desktop host between them (a relay, or iroh in the
  browser, would be the next step). A page served over https may only open `wss://`,
  which needs TLS in front of the host.
- Live links to the engines, MCP and anything that starts a process are unavailable.

## Next steps

1. Live sessions, still open: two browsers meeting without a desktop host (a relay
   server, or iroh in the browser); `wss://` for a page served over https (TLS in
   front of the host); the automatic rejoin after a dropped link, not yet tried in a
   browser.
2. Not yet seen in a real browser, only in headless Chromium or not at all: Firefox and
   Safari, saving in place through File System Access, Edit > Paste (headless refused the
   clipboard), render_wgpu's texture decoding on WebGPU, touch on a tablet, and the
   autosave (every ten minutes).
3. All startup data is still fetched before the editor starts. Measure a fresh
   release module and cold/warm startup; archive packing reduces request count,
   but does not implement lazy loading or prove a startup-time improvement.
4. The GL renderer on Android GLES GPUs, which the shader rewrite now allows.
5. Move expensive full export work off the web UI thread. Measured maps and the
   implemented asset deltas/cache are documented in `performance.md`.

## Playtest in a second tab

File > Play in a new tab opens `play.html` on the same origin. Keep the editor tab
open. Each editor tab creates a unique BroadcastChannel; viewers cannot accidentally
receive another editor's map. Opening/reloading a viewer requests the latest good
export, including unsaved geometry, GLBs, textures, collision and animation.

Changes are exported in memory after 300 ms without another content edit. Within
one map and runtime template, the viewer stages a scene update in the same iframe.
Only changed GLBs/textures and changed entity fields travel over BroadcastChannel;
unchanged assets are reused. Collision trees are rebuilt before committing. Player
position, look, velocity, mouse capture and compatible animation clocks/trigger state
survive. Triggers whose definitions change reset; animations retain time/playing/pose
when their version, selected clip and autoplay remain compatible. Removed objects
release GPU resources. A new map, scale or runtime template uses a fresh iframe.
Failed exports/imports retain the last playable scene and report the error.

The editor still builds a full export before computing a delta. This reduces transfer,
decoding and renderer uploads, but does not make the authoring export incremental.
Large textured maps can still stall the UI; native timings are not browser timings.

Three.js 0.186.0 and the required addon modules are bundled with the web application
under `vendor/three`, with their MIT license. The playtest makes no CDN requests;
standalone exported HTML retains its existing CDN behavior. This works with a
locally served build without installing an engine or running an online service.

The generated runtime and tab protocol have a reproducible Chromium check at
`examples/engine-checks/threejs/verify_web_playtest.cjs`. Firefox/Safari, real touch
input and heavy-map frame latency remain separate verification tasks.

Verified on 14 October 2026 in local Chromium with external requests blocked:
initial level, exact player placement, animation/trigger snapshot preservation,
unchanged mesh/texture reuse, one changed valid GLB, rapid entity removal/restore,
updated static collision,
channel isolation, failed-import retention, map reset, viewer reload and close.
With `--editor`, the check also starts the complete WASM application, opens the
bundled motion_lab and exports 10 entities with collision through its actual
File menu. Desktop and Trunk WASM builds passed offline. Use the existing local
Node/Playwright/Chromium installation; no package installation is required.

```sh
cargo build -p heron-brush --offline
python3 examples/engine-checks/export_motion_lab.py three
# From crates/heron-brush-web, with the matching cached wasm-bindgen on PATH:
CARGO_NET_OFFLINE=true trunk build --offline=true --skip-version-check
# From the repository root, with existing Playwright and Chromium configured:
node examples/engine-checks/threejs/verify_web_playtest.cjs /tmp/heron-motion-three/motion_lab.html --editor
```

If Trunk reports a missing wasm-bindgen while offline, point PATH to its already
installed matching version. Do not install or download tools to run this check
on a restricted connection. Some Trunk versions reject `NO_COLOR=1`; unset that
variable for the build if it reports this parsing error.

## Save failures and release checks

Browser-storage writes now remain pending until the IndexedDB transaction commits.
Writes and deletions to the same path run in order. Pending and failed saves keep
the tab-close warning active even after the editor considers a map saved. A failed
write retains its bytes and shows Retry storage and recovery-download buttons.
Recovery copies are held in this tab's memory, so keep it open until recovery is
complete. This status describes browser storage; it does not claim a downloaded
file was written to disk.

The bridge tests run with `node --test scripts/tests/web-storage.test.mjs`. A real Chromium IndexedDB check passed failure UI, retry and reload persistence;
quota failure was injected rather than filling the disk. Same-basename isolation
is also covered. Run `python3 scripts/tests/serve_web_storage.py` and open its
printed URL in an installed browser to repeat this check. Actual quota exhaustion,
File System Access picker/write UI, Firefox and Safari remain pending.

## Measuring a release locally

`python3 scripts/serve_web_release.py /tmp/heron-web-release --metrics` serves an existing
Trunk build on a fresh localhost port. It precompresses files with gzip-6, serves
WASM with application/wasm, varies cache entries by Accept-Encoding, and supplies
ETags. Hashed JS/WASM assets are immutable; mutable index data and HTML require
validation. This diagnostic server does not configure the eventual public host.
Measure both a new origin and a reload in the same installed browser. Record
transferred bytes, cache hits, load-to-editor time and renderer, rather than
claiming production performance from archive size alone.

Selected-file writes also run in order and keep the close warning active until
the writable stream closes. On a failed write the bridge aborts the stream,
requests a recovery download, and displays a notice explaining that the original
file was not updated. A successful download request is not proof of a disk write.
The browser check additionally exercises real OPFS writable streams through an
isolated handle adapter; it does not test the external file-picker permission UI.

The metrics option adds only a diagnostic HTML probe; it does not alter the built
WASM. `document.documentElement.dataset.heronStartup` contains loader-removal
time, resource transfer sizes and the browser's reported JS heap. These figures
do not represent total process memory or a measured input-latency budget. Current
release measurements and the subsequent content-addressed cache fix are recorded in
[performance](performance.md#web-measurements).

The startup index is revalidated on each load. Its archive URL includes the exact
SHA-256 and the loader requests it with force-cache. A changed package gets a new
URL, so cached old data cannot satisfy the new manifest. The loader verifies
length and SHA-256 before exposing any files. A failed cached response is retried
once with reload; corrupt network data remains a visible failure.

Run `python3 scripts/tests/serve_web_bundle_cache.py` and open the printed URL to
repeat the real-browser cache check. The disposable page reloads automatically
and requires a zero-byte archive transfer with unchanged file count. This verifies
the loader/cache contract without recompiling WASM. It does not prove an entirely
offline page load or that a public host sends the correct headers.

## Isolated playtest exporter

The web release includes a separate `heron-brush-export-web` WASM module, loaded
on demand by `playtest_worker.mjs`. It receives a versioned authoring snapshot,
loaded entity definitions/export rules and changed virtual files. Geometry
export, HTML extraction and delta preparation run in the worker. The editor
still prepares the snapshot synchronously, which needs its own large-map timing.

The controller terminates busy stale jobs and ignores late results from replaced
workers. A replacement receives all files plus the last published viewer baseline.
Unchanged images are transferred only on the first job or after a restart.
Transferred buffers are copies, never views into the editor's WASM memory.
Worker errors reset the worker and appear in the editor console.

`stage-web-worker.py` writes a versioned worker manifest after Trunk builds both
modules. Deploy the whole dist, including the worker's snippets. Source checks
alone do not verify worker boot or native/wasm resource parity.

For a local built release, generate a native reference and compare its actual WASM
export without installing a browser:

```sh
cargo run -p editor_core --example web_playtest_fixture --offline -- /tmp/heron-worker-motion
node scripts/tests/check_export_worker.mjs /tmp/heron-web-release /tmp/heron-worker-motion
```

Add `pbr` after the output directory to generate the imported GLB reference.
`python3 scripts/tests/serve_web_worker_check.py RELEASE FIXTURE` serves a separate
browser test for real worker boot, native level parity, live patches, cancellation,
restart and retained viewer state. Click its Start button to open the viewer.
Restart this server after changing artifacts because it precompresses them at boot.


## Picker fallback checks

The production bridge now distinguishes unavailable File System Access from user
cancellation. Its in-page copy dialogs support Cancel/Escape and filename
validation without native prompts or HTML network form submission. Read failures
stay visible; a new fallback operation cancels an earlier one. Native picked
handles are cleared before a new selection, preventing an abandoned operation
from becoming the next copy's handle.

Run `node --test scripts/tests/web-storage.test.mjs scripts/tests/web-picker.test.mjs`.
On 16 October all 21 cases passed: 13 storage/export bridge cases and 8 picker
cases. The portable CI workflow includes both files; it has not run remotely.

`python3 scripts/tests/serve_web_picker.py` serves a disposable opaque-origin
frame with the real bridge and native browser APIs. In installed in-app Chromium,
both APIs genuinely threw SecurityError. Before the fix the bridge silently
returned null. Afterward both copy dialogs appeared; Save validation, Save,
Cancel and Escape worked. A generated 53-byte map actually downloaded with the
exact expected bytes. The test deliberately omits allow-forms and allow-modals,
so the fallback relies on script buttons and DOM dialogs, not blocked HTML form
submissions or native prompt windows. A screenshot of the dialog was inspected.

The attempted filechooser automation timed out and the input reported cancellation.
OS selection into the fallback file input remains unverified. File reading and
read failures are covered by Node tests, not by that UI attempt. Direct native
File System Access success/permission UI, the complete WASM editor flow, Firefox
and Safari remain separate checks. Browser evidence is in
`/tmp/heron-picker-browser-evidence.json`; regression output is in
`/tmp/heron-picker-final-tests.log`. The generated download was removed after
checking its hash, timestamp and absence of symlinks. Test tab/server were closed.
The installed editor and existing web build were not updated for this change.


## Folder import reliability and read concurrency

Folder selection now starts at most four `File.arrayBuffer` requests at a time.
Results retain the browser's original order, relative paths and bytes even when
reads finish out of order. All paths are validated before reading: each must
have a folder root and a nonempty relative path, without traversal segments or
backslashes, and every file must share the same selected root.

A rejected read stops new requests, lets the other in-flight reads settle and
returns null with a visible "No files were imported" notice. Successful earlier
reads are not returned as a partial folder. Failure to access the file list also
completes with a visible error rather than leaving the async request pending.
The Rust importer receives no array on failure, so its per-file store loop does
not run. A successful retry clears the folder's old notice while retaining an
unrelated file notice. Cancel and an empty selection remain cancellation.

This bounds pending read requests and their promises. The completed result still
retains the whole folder payload required by the current Rust bridge. Streaming
imports, total browser RAM and real folder-picker performance remain unverified.
There is no claimed memory percentage or real-disk throughput improvement.

The combined storage/picker suite passed 30 tests locally. Nine folder cases
cover controlled out-of-order IO, a maximum of four pending requests, immediate
and late failures, retry, unusable paths, file-list access failure, cancellation and notice ownership across overlapping imports.
Two targeted tests fail against an isolated copy of the preceding committed
bridge: it starts 24 reads in the controlled 24-file fixture and rejects the
file-list callback rather than completing the outer request. Production source
was not changed during that negative control.
Evidence: `/tmp/heron-folder-import-final.log`, `/tmp/heron-folder-negative.log`.
This is controlled File API testing, not a completed OS folder-picker check.

An older successful import does not dismiss a newer failure notice, even when its
text is identical to the previous failure. A focused negative test caught this
case during implementation; the final full suite passes it. The negative output
is in `/tmp/heron-folder-notice-negative.log`.
