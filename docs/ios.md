# Native iPad prototype

Heron can be built as a native iPad application using eframe and wgpu's Metal
backend. This is an experimental simulator port, not an App Store release.

## Build and run on an Apple Silicon Mac

Prerequisites: Xcode with an installed iOS Simulator runtime, Python 3.11 or newer,
and Rust's `aarch64-apple-ios-sim` target. The build script does not download runtimes
or dependencies. It uses one Cargo job, disables debug symbols and incremental
compilation, requires 1 GiB free before starting, and stops compilation below
512 MiB free. A first build can require substantially more space than these guards;
leave several GiB available for the target cache and simulator.

```sh
scripts/build-ios-simulator.sh
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
xcrun simctl list devices available
# Replace DEVICE with the UUID of an installed iPad simulator.
xcrun simctl boot DEVICE
xcrun simctl bootstatus DEVICE -b
xcrun simctl install DEVICE 'target/ios-simulator/Heron Brush.app'
xcrun simctl launch DEVICE io.github.mojzisekjan.HeronBrush
open -a Simulator
```

The generated bundle is ad-hoc signed for the simulator. Physical-device builds,
provisioning and distribution are not provided by this script.

## Storage and interface

The app reads bundled assets and game definitions from its read-only application
bundle. Preferences and autosaves use its sandbox under Library/Application Support;
user maps can be saved in Documents. The in-app file browser offers Documents and
Examples. The bundle enables document sharing so the Documents folder can be exposed
by the system's Files integration. Direct document-open events and an external
security-scoped document picker are not implemented yet.

The shared touch interface provides tap selection, long-press context menus,
one-finger orbit, two-finger pan and pinch zoom, plus modifier toggles for editing.
Desktop file dialogs and native OpenGL window creation are excluded on iOS.
Secondary editor windows use eframe's embedded mobile viewports.

## Simulator validation

Tested on the installed iPad Pro 11-inch (M5), iOS 26.5 simulator:

- Native launch and the Dragon example rendered in the 3D and orthographic views.
- Portrait and landscape layouts, menu targets and file dialogs.
- Save as into Documents, then reopen the saved map through the file browser.
- Eleven host-side gesture tests, including dialogs above map views, coalesced
  start/end drags, final movement on release and cancellation; two keyboard geometry
  tests cover rotation, interface zoom, floating/hidden keyboards and invalid input.
- A fast one-finger drag rotates the 3D view without selecting geometry.
- Software keyboard automatically opens on text focus with the simulator's hardware
  keyboard disconnected. Touch typing and backspace update the field.
- The editor and file dialog fit above the docked keyboard; hiding it restores the
  full canvas. Rotating with the keyboard open preserves usable controls.
- Entered a new filename using only the software keys and saved `ipad.map` in
  Documents with the Save button still visible above the keyboard.
- 37 headless GPU tests with `cube-array-fallback` enabled (six opt-in tests skipped).

The simulator's Metal adapter lacks cube texture arrays. Its renderer uses six
2D-array layers per cube and explicit face lookup for local probes and point-light
shadows. Filtering across cube-face seams is not identical to hardware cube sampling;
physical-device and desktop builds retain the normal cube-array path. The fallback
can be exercised on desktop with:

```sh
CARGO_BUILD_JOBS=1 cargo test -p render_wgpu --features cube-array-fallback \
  --test headless -- --test-threads=1
```

The vendored winit patch makes iOS drawable dimensions agree with touch coordinates;
see `vendor/winit/HERON_PATCH.md`. Safe-area padding remains supplied by egui-winit.

## Remaining device validation

A simulator does not establish physical-device GPU performance, memory limits,
Apple Pencil support, complex text composition, background recovery or Files/iCloud
interoperability. These must be tested on an actual iPad before calling the port
supported. Desktop engine build/run tools also require platform-specific alternatives
on iPad; their presence in the shared interface does not make them available on iOS.

Multi-touch pan/zoom and long-press context menus have automated gesture coverage,
but still need end-to-end manual checks on an actual device. Basic one-finger orbit
and keyboard entry were verified in the simulator. Floating keyboard placement and
IME/composition (including non-Latin keyboards) remain unverified.

The keyboard bridge uses UIKit's keyboard layout guide (iOS 15+, deployment target
16) without altering Metal drawable dimensions or touch coordinates. It preserves
system safe-area insets and schedules a repaint every 100 ms during text focus or
keyboard dismissal, even when no other input arrives. Opening another map retains this window-level state.

## iPad workspace

The iOS build uses a separate document header and editing shelf instead of the
desktop menu and dense icon toolbar. The header exposes Open, Undo/Redo, document
actions, command search, More and the inspector toggle. The shelf switches between
3D, Top, Front, Side and all four views, selects tools, changes grid spacing, frames
the selection and saves. New documents start in 3D. Active tool settings remain
available in a contextual row for Brush, Fillet and Sculpt, shared with the desktop
implementation. Tools without inline options leave that space to the canvas.

Controls and menu rows use 44-point touch targets. On a wide screen the inspector
stays on the right; in portrait/narrow windows with enough height it docks below
the canvas. The technical console strip is hidden until the console is requested.
The workspace still uses egui; it is not a UIKit/SwiftUI replacement. The welcome
screen, native document picker and replacement of keyboard-modifier toggles are
further work.

Simulator checks covered the portrait/landscape arrangement, Top and four-view
switches, the tool menu and Brush settings. The document-header icons, dropdown
chevrons and inspector toggle were also checked in the installed simulator build.

The rounding regression fixture now distinguishes a valid radius-13/nine-segment
join from a radius-15/six-segment join that overlaps only after grid snapping. Tests
retain the refusal message and location checks, verify that refusal leaves the map
unchanged, and cover applying and undoing the valid case. All 62 corner-join tests
pass on the host. The full application unit suite also passes: 206 passed, five
tests ignored, no failures.

The final installed build was checked in both orientations: Rotate leaves no empty
options row, while Brush and Fillet keep their controls. Typing `fill` with the
software keyboard finds Fillet; rotating with the keyboard open preserves the query
and results. Choosing Fillet dismisses the keyboard and restores the full workspace.
