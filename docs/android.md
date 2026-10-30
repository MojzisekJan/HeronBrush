# Heron Brush on Android

Status, 2026-10-10: **basic support, runs on a device.** The desktop application builds as
an Android library (`crates/heron-brush-android`) with a NativeActivity entry point, and
`scripts/build-android.sh` packages it as an APK. A release APK runs on a TCL NXTPAPER 14
tablet (Android 15), installed by downloading it from the development Mac over Wi-Fi.

## How it is put together

| Part | Where | What it does |
|---|---|---|
| Shared application | `crates/heron-brush/src/lib.rs` | The whole editor; `main.rs` (desktop) only calls `run_desktop()` |
| Android library | `crates/heron-brush-android` | A `cdylib` whose `android_main` calls `heron_brush::android::run`. A crate of its own so that only Android links the editor as a shared library; elsewhere it is empty |
| Platform glue | `crates/heron-brush/src/android.rs` | Logcat logging, storage directories, unpacking bundled data, the file browser's places, the first-run layout, the soft keyboard, the safe area |
| Files | `crates/heron-brush/src/file_dialog.rs` | System dialogs (rfd) on the desktop; an in-app file browser and message box on Android |
| Touch | `crates/heron-brush/src/touch.rs` | Gestures over the map views; bigger hit areas |
| Packaging | `android/AndroidManifest.xml`, `android/res`, `scripts/build-android.sh` | The APK, without Gradle or Java code |

Choices:

- **NativeActivity, not GameActivity.** eframe 0.36 supports both through winit 0.30.
  NativeActivity needs no Java/Kotlin code, no AAR and no Gradle; GameActivity would add a
  Java dependency and a C++ build for a better IME. eframe's `accesskit` feature only
  works with GameActivity, so accessibility is desktop-only for now.
- **wgpu, Vulkan first.** The window and the 3D view run on wgpu as on the desktop. egui
  draws on Vulkan or GLES; the 3D renderer (`render_wgpu`) needs storage buffers in the
  vertex stage, so on a GPU that only offers GLES the 3D view shows a message instead
  (the 2D views and the rest of the editor work). On Android the wgpu device asks for the
  adapter's own limits, not wgpu's desktop defaults, so a phone GPU that falls short in an
  unused limit still opens. A GLES path for the 3D view is planned in
  `docs/renderer-wgpu.md`, *Not done yet*.
  The GL build (`--no-default-features`) uses desktop GLSL and is not for Android.
- **No Gradle.** `scripts/build-android.sh` calls cargo-ndk, then the SDK's `aapt2`,
  `zipalign` and `apksigner` directly. A Gradle project can replace it later (signing for
  release, Play bundles) without touching the Rust side.
- **Not edge-to-edge** (`targetSdk 34`): the system lays the activity out between the
  status and navigation bars and resizes it for the soft keyboard (`adjustResize`), so the
  app needs no inset handling of its own. Whatever the activity's content rectangle still
  leaves out is passed to egui as the safe area. Rotation and other configuration changes
  are handled in place (`configChanges`), so the editor and the open map survive them.

## Building

### One-time setup (macOS)

Sizes are approximate. The Homebrew formula `openjdk@17` works as well as the Temurin cask.

| What | How | Download | On disk |
|---|---|---|---|
| JDK 17 (apksigner, keytool, sdkmanager) | `brew install --cask temurin@17` | 190 MB | 330 MB |
| Android command-line tools (sdkmanager) | `brew install --cask android-commandlinetools` | 150 MB | 300 MB |
| platform-tools (adb) | `sdkmanager "platform-tools"` | 15 MB | 40 MB |
| build-tools (aapt2, zipalign, apksigner) | `sdkmanager "build-tools;35.0.0"` | 60 MB | 150 MB |
| platform (android.jar) | `sdkmanager "platforms;android-35"` | 65 MB | 130 MB |
| NDK (linker, sysroot) | `sdkmanager "ndk;27.2.12479018"` | 0.7–1 GB | 2–2.5 GB |
| cargo-ndk | `cargo install cargo-ndk` | – | 10 MB |
| Rust targets | `rustup target add aarch64-linux-android x86_64-linux-android` | 2 × 60 MB | 2 × 200 MB |
| *Emulator (optional)* | `sdkmanager "emulator"` | 300 MB | 450 MB |
| *System image (optional)* | `sdkmanager "system-images;android-35;google_apis;arm64-v8a"` | 1.5 GB | 3–4 GB |
| *Virtual device (optional)* | `avdmanager create avd …` | – | 2–6 GB as it fills |

Without the emulator that is about 3.5 GB; with it about 10 GB. Android Studio bundles all
of it (and more, about 8–10 GB) if a GUI is preferred. The build directory adds about
1–2 GB per ABI for a debug build.

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"   # where sdkmanager installs
yes | sdkmanager --sdk_root="$ANDROID_HOME" --licenses
sdkmanager --sdk_root="$ANDROID_HOME" "platform-tools" "build-tools;35.0.0" "platforms;android-35" "ndk;27.2.12479018"
cargo install cargo-ndk
rustup target add aarch64-linux-android x86_64-linux-android
```

On Linux the same `sdkmanager` packages apply; the JDK comes from the distribution.

### Build, install, run

```sh
scripts/build-android.sh --abi arm64-v8a          # a phone, or the emulator on Apple silicon
scripts/build-android.sh --abi x86_64             # an x86_64 emulator image (Intel/AMD hosts)
scripts/build-android.sh                          # both ABIs in one APK
scripts/build-android.sh --abi arm64-v8a --run    # build, adb install -r, start
scripts/build-android.sh --release                # optimised; still signed with the debug key
scripts/build-android.sh --check                  # type-check only: needs no SDK or NDK
```

The APK is `target/android/heron-brush-debug.apk` (package
`io.github.mojzisekjan.HeronBrush`, minimum Android 8.0 / API 26). Install it by hand with
`adb install -r target/android/heron-brush-debug.apk`; start it from the launcher or with
`adb shell am start -n io.github.mojzisekjan.HeronBrush/android.app.NativeActivity`.

Logs: `adb logcat -s HeronBrush RustStdoutStderr '*:E'` (`HeronBrush` is the app's own
tag; Rust panics are logged there too). The editor's console pane shows the same
messages as on the desktop.

An emulator on an Apple-silicon Mac:

```sh
sdkmanager "emulator" "system-images;android-35;google_apis;arm64-v8a"
avdmanager create avd -n heron -k "system-images;android-35;google_apis;arm64-v8a" -d pixel_7
emulator -avd heron -gpu host &          # Vulkan through the host GPU
scripts/build-android.sh --abi arm64-v8a --run
adb exec-out screencap -p > shot.png
adb emu kill
```

### Trying the Android behaviour on a desktop

The touch layout, gestures and in-app file browser are not Android-only; these variables
turn them on in a desktop build:

```sh
HERON_BRUSH_TOUCH=1 HERON_BRUSH_IN_APP_FILES=1 HERON_BRUSH_WINDOW_BUTTONS=none \
  cargo run -p heron-brush -- --new-instance assets/crate.map
```

`HERON_BRUSH_SCRIPT` scripts (`src/script.rs`) have finger steps (`tap`, `hold`, `swipe`,
`pinch`, `pan2`) for replaying gestures. A real touch screen on Windows or Linux turns the
touch layout on by itself at the first touch.

## What works, and how it differs from the desktop

Runs on a TCL NXTPAPER 14 tablet (Android 15), installed by downloading the APK.

**Touch** (`touch.rs`). A touch that lands in a map view is taken out of egui's emulated
mouse and read as a gesture:

| Gesture | 3D view | 2D views |
|---|---|---|
| Tap | Click: select | Click: select |
| Long press (0.5 s, still) | Right-click: context menu | Right-click: context menu |
| One finger drag | Orbit about the point straight ahead | Left-button drag: move the selection, draw a brush, marquee |
| Two fingers | Pan and pinch-zoom | Pan and pinch-zoom about the fingers |

**Shift, Ctrl and Alt** are toggles in the bottom-left corner of the views. A key stays
held until it is tapped again, and every gesture carries it: Shift and a drag from a
selected brush's edge resizes it (extrudes the face), as Shift-drag does with a mouse.
While a key is held, one finger in the 3D view drags as a mouse does instead of orbiting,
so a face can be taken hold of there too. Lists (assets, entity classes) scroll with a
finger; a double tap places what a cell holds.

A second finger during a 2D drag cancels the drag (nothing is committed, nothing to
undo). Touches anywhere else go to egui unchanged: buttons, fields, scrolling lists.
Editing geometry by touch happens in the 2D views; the 3D view navigates, selects and
opens the context menu. With a touch screen egui's interaction radius grows, scroll bars
widen and tooltips wait longer.

**Layout.** The first run uses the single-view layout. A small screen starts with the
views alone; the header's inspector button (right end) shows or hides the inspector, which
sits beside the views in landscape and under them when the screen is upright and narrow
(its divider can be dragged). Tapping a view's name pill steps to the next view (Space on
a keyboard; also View > Next view). When the window is too narrow for the menu bar, the
menus fold into one ☰ button. There are no window buttons. The 3D view cannot be
detached into a window of its own.

**Files and storage.**

- The profile (preferences, recent maps, examples, autosaves, caches) lives in the app's
  internal storage (`/data/data/io.github.mojzisekjan.HeronBrush/files/config/heron-brush`),
  set through `editor_core::preferences::set_platform_directories`. `TMPDIR` points to
  `files/tmp`.
- The examples are written there on first use, as on the desktop. The data the desktop
  package ships beside the executable (`assets/` for the asset browser, `gamedata/` as
  `games/`) is packed into the APK's assets with an index and copied to `files/bundle` on
  the first run after each install or update.
- File > Open, Save As, point and portal files, entity definitions, materials, WADs,
  assets, plugins, exports and render output use the in-app browser. Its places: *Heron
  Brush* (`files/maps`, the default for new maps), *Examples*, *Documents* (shared
  `Documents/Heron Brush`, created when allowed), *Download* (shared, when readable) and
  *Android/data* (the app's folder on shared storage, which a computer sees over USB). A
  tap picks a file, a second tap (or Open) takes it; saving has a name field.
- Under Android 11+ scoped storage the app sees its own files in Documents and Download
  but not other apps' files there; maps copied in over USB belong in *Android/data*. The
  Storage Access Framework (opening any document a provider offers) is not implemented.
- Unsaved changes are asked about in an in-app box (Save / Discard / Cancel). Removing a
  map from the welcome list on Android only takes it off the list (there is no wastebasket,
  and a map is never deleted outright).

**Keyboard.** The soft keyboard is asked for explicitly whenever a text field has the
keyboard focus and hidden when none has; the activity resizes above it. NativeActivity
delivers what the keyboard types as key events, so plain text works, but composing input
(predictive text, CJK input methods) does not. A hardware keyboard works as on the desktop.

**Off or not useful on Android:** the single-instance hand-over (not compiled into the
Android entry), the command line and stdio MCP (`--mcp`), engine live links (they write into
desktop engine projects; the menu entries remain), collaboration over iroh (not in the
default build). Live sessions on the local network (TCP) and the MCP endpoint on
127.0.0.1 are compiled in and should work; untested.

**Lifecycle.** eframe keeps the wgpu device across suspend and resume and recreates only
the surface, so the 3D view's GPU resources survive backgrounding. Autosave runs as on
the desktop. The process exits when the activity is destroyed (winit cannot create its
event loop twice in one process); a document not saved by then is in the autosave.

## Known limitations

- Run on one tablet only, without logs (no USB debugging); not on an emulator or phone.
- The 3D view needs Vulkan (most arm64 devices since 2018; the emulator with
  `-gpu host`). render_wgpu samples floating-point targets (`Rgba16Float`, `Rgba32Float`)
  and uses depth cube arrays; phone GPUs that lack a format feature will fail validation
  rather than degrade. Measure memory and frame time with the post-processing on a real
  device, and lower *Shadow map size* / turn off GTAO, SSR and TAA if needed.
- The editor's controls take Material's sizes here: 40 points tall with 14-point text
  (`theme::CONTROL_HEIGHT`), where the desktop has 28 and 13, and egui's hit radius grows.
  Comfortable with a fingertip on a tablet, still tight on a phone. The toolbar does not
  wrap: a phone held upright shows only part of it, landscape is recommended.
- No Storage Access Framework, so no `content://` documents, no "Open with" from other
  apps, no persistable grants.
- No IME composition; no accessibility (accesskit needs GameActivity).
- Not edge-to-edge (targetSdk 34): Google Play requires 35 for new apps, which forces
  edge-to-edge; then the safe area needs the window insets (JNI), not only the content
  rectangle.
- Release APKs are signed with the key in `HERON_ANDROID_KEYSTORE`; an `.aab` for Play needs
  the Gradle route or `bundletool`.

## Checklist for the first run

1. `scripts/build-android.sh --abi arm64-v8a --run` on the emulator (`-gpu host`) and on a
   phone; `adb logcat -s HeronBrush RustStdoutStderr '*:E'` while it starts.
2. The welcome screen shows; open an example; the 3D view draws (Vulkan adapter named in
   the log); rotate the device and come back from the home screen.
3. Tap selects, long press opens the menu, one finger orbits, two fingers pan and zoom;
   a drag in a 2D view moves a brush, Undo puts it back, a second finger cancels a drag.
4. A text field opens the soft keyboard and typing reaches it; Save As into *Heron
   Brush*, reopen from the welcome list and through File > Open.
5. The asset browser lists the bundled assets; the game *heron_brush* is found (Preferences).
6. Screenshots: `adb exec-out screencap -p > shot.png`.

## Repeatable checks without a device

```sh
rustup target add aarch64-linux-android x86_64-linux-android
scripts/build-android.sh --check                  # the app, both ABIs
python3 scripts/check_portable_core.py --offline  # the six core libraries, no UI deps
cargo test -p heron-brush touch:: file_dialog::   # gestures, browser
```

`check_portable_core.py` resolves the target-specific dependency graph, rejects desktop UI
libraries reachable from the core crates and runs `cargo check --lib` for them.
`.github/workflows/portable-core.yml` runs it and the app's Android type-check on pushes.
These need neither the SDK nor the NDK, and they do not link anything.

## Remaining work

1. **First device runs** (above), then fix what they find.
2. **Storage Access Framework**: open/save through document providers with opaque
   document identifiers and stream I/O (a JNI bridge or a small Kotlin shell); keep
   internal working copies and report success only after the provider write finishes.
3. **GLES path for the 3D view** (`docs/renderer-wgpu.md`, *Not done yet*):
   uniform-buffer or data-texture tables chosen from `DownlevelCapabilities`, ETC2/ASTC textures, power-aware presets.
4. **Edge-to-edge and insets** for targetSdk 35, then a release keystore and Play bundle.
5. **Touch editing in 3D**: drag handles and the gizmos by touch (picking tolerances for
   fingers), stylus pressure, a tool row that fits a phone.
6. **IME and accessibility** (GameActivity, or a text-input bridge).
