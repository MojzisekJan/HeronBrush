# Performance

How the editor's speed on big maps is measured, how to repeat the measurements, what
they show today, and where it is still slow. The renderer's own design and profiling are
in [the renderer doc](renderer-wgpu.md).

## Workloads

The benchmark maps are generated in code from a seed (`editor_core::bench_support`,
behind the `test-support` feature), so no large files are committed and every run sees
the same data: 1k / 10k / 50k brushes (boxes, cylinders and arches over four layers, with
groups, brush entities and lights), 200 keyframed `func_animate` entities, 5,000 nested
groups, 500 linked copies of a 50-brush group, 20,000 point entities, 200 layers, and
folders of 2,000 / 5,000 / 10,000 PNG textures.

## Running the benchmarks

The benchmarks are `#[ignore]`d tests that print Markdown tables (median and p95 of N
runs). They use a plain `Instant` harness rather than Criterion: most cases need long
setup, mutate a shared document, and look for large effects, not 2% ones.

```sh
# Editor core: load/save, selection, transforms, geometry tools, scene and flat-view
# building, validation, export, animation, materials, surfaces while editing.
cargo test --release -p editor_core --features test-support --test perf -- \
    --ignored --test-threads=1 --nocapture

# Special cases: groups, linked groups, big transforms, entities, layers, a 512-sided
# brush, history growth, material folders.
cargo test --release -p editor_core --features test-support --test perf_special -- \
    --ignored --test-threads=1 --nocapture

# The egui side, headless: flat views, the material browser, the Map and Entity tabs.
cargo test --release -p heron-brush perf_egui -- --ignored --nocapture --test-threads=1

# One area, smaller maps, fewer runs:
HERON_BENCH_SIZES=1000,10000 HERON_BENCH_RUNS=3 cargo test --release -p editor_core \
    --features test-support --test perf -- --ignored --nocapture perf_selection_and_transforms

# Materials, one size per process so peak RSS means something:
HERON_BENCH_TEXTURES=5000 cargo test --release -p editor_core --features test-support \
    --test perf -- --ignored --nocapture perf_materials
```

| variable | meaning |
|---|---|
| `HERON_BENCH_SIZES` | brush counts, default `1000,10000,50000` |
| `HERON_BENCH_RUNS` | repetitions (default 5, at most 3 on 50k) |
| `HERON_BENCH_TEXTURES` | texture counts, default `2000,5000` |
| `HERON_BENCH_ONLY` | run only benchmarks whose name contains this (for attaching a profiler) |
| `HERON_BENCH_DIR` | where temporary maps and textures go (default: the system temp directory) |
| `HERON_BENCH_KEEP` | keep the temporary data |

Other suites:

- **Renderer**: `cargo test --release -p render_wgpu --test headless bench_big_maps --
  --ignored --nocapture`; see the renderer doc for its variables.
- **Export** (Criterion, on the bundled dragon map): `cargo bench --locked -p
  scene_export --bench export`, with groups for parsing, level builds, the scene writers
  and in-memory GLB/OBJ mesh serialization. They use different build options; do not add
  them up as an export time.
- **Map loading**: `crates/map_model/examples/map_load_probe.rs` times parsing (`parse`)
  or parsing plus document-tree construction (`load`, or `borrowed` for the borrowing
  `build_tree` path). `scripts/tests/generate_map_load_fixtures.py <dir>` writes deterministic
  fixtures; `scripts/tests/benchmark_map_loading.py <before> <after> <fixtures> <out.json>
  --cpu N` alternates two prebuilt probes, one process at a time, pinned to one CPU
  (Linux, `taskset`). Omit `--cpu` on macOS; processes still alternate serially.
- **Archives**: `cargo test -p map_model --lib bench_archive_residency -- --ignored
  --nocapture --test-threads=1` (and `bench_zip_archive_residency`) measures RSS after
  mounting a sparse PAK (ZIP) with large unused members, sized by `HERON_BENCH_PAK_MB`
  (`_ZIP_MB`); `HERON_BENCH_PAK_MEMORY=1` (`_ZIP_MEMORY=1`) forces the whole-archive
  reader. On Linux they fail if RSS grows by half the archive; CI runs both.
  `profile_indexed_game_asset_lookup` reads 20,000 small Deflate members.
- **WASM archive sharing**: `scripts/tests/check_web_archive_sharing.cjs` runs the
  `web_archive_residency` example of `map_model` (feature `archive-bench`, built for
  wasm32 with `wasm-bindgen --target nodejs`) and checks that mounting does not copy the
  archive.

## Pointer picking and duplication

```sh
cargo run --release -p editor_core --example interaction_latency
```

This probe reports median and p95 latency for pointer hits and misses on 32 / 256 /
2,048 curved patches, duplicating 1,000 / 10,000 selected boxes, and orthographic
picking through 1,000 / 10,000 stacked boxes. Scene construction and Undo are outside
the timed region. Picking uses 50 samples per patch case, and duplication and stacked
picking use five samples each. The stacked boxes are a worst case where every brush
intersects the ray, rather than a typical room layout.

Measured on this workstation on 2026-11-03, with both release binaries pinned to
one CPU. Values below are medians of three runs per binary, alternating the old and
new versions; each run uses the sample counts above. These measure core operations,
not complete editor frames or GPU rendering.

| operation | before (ms) | after (ms) |
|---|---:|---:|
| 2,048 patches, ray misses | 11.960 | 0.102 |
| 2,048 patches, ray hits one | 11.725 | 0.106 |
| Duplicate 10,000 boxes | 238.626 | 75.118 |
| Flat pick through 10,000 stacked boxes | 315.210 | 9.606 |

Picking rejects patches outside their control-point bounds before tessellation, with
origins inside the bounds still tested against the surface. Brush picking carries
layer and brush references through the traversal instead of looking them up again
for every hit. Bulk duplication resolves closed-group membership in one tree walk.

## Appearance edits and nested copies

```sh
cargo run --release -p editor_core --example attribute_latency
```

The probe measures UV changes, painting one face and copying texture attributes on
64 / 256 / 512-sided cylinders, a UV change on 10,000 boxes, copying 1,000 boxes
inside 0 / 8 / 32 nested groups, and copying one box beside 9,999 unrelated groups.
Each operation has five timed samples; construction, selection and Undo are outside
the measured region. Only changed faces and nodes are committed, so every sample
must make one undoable edit.

Appearance edits retain the existing hull only when the face planes, defining points
and face order are unchanged and the hull fits the requested world bounds. Other
edits use the existing rebuild and validation path. Duplication copies child nodes
once and allocates replacements for existing containers only after finding a changed
descendant.

Measurements on 2026-11-03 used release binaries pinned to the same CPU, with no
concurrent test run. Each value below is the median of three runs of five samples;
the old and new binaries ran in alternating order. These are document-operation
timings, excluding rendering and UI event handling.

| Operation | Before | After |
| --- | ---: | ---: |
| UV edit, 512-sided cylinder | 18.839 ms | 0.273 ms |
| Paint one face, 512-sided cylinder | 13.552 ms | 0.187 ms |
| Transfer texture, 512-sided cylinder | 14.452 ms | 0.245 ms |
| UV edit, 10,000 boxes | 190.969 ms | 82.401 ms |
| Duplicate 1,000 boxes, no group | 6.309 ms | 5.092 ms |
| Duplicate 1,000 boxes, group depth 8 | 28.750 ms | 5.812 ms |
| Duplicate 1,000 boxes, group depth 32 | 96.314 ms | 4.844 ms |
| Duplicate one box beside 9,999 groups | 16.568 ms | 7.458 ms |

## MCP selection requests

```sh
cargo run --release -p editor_core --example mcp_latency
```

The probe sends `tools/call` requests to the same `mcp::respond` handler used by the
stdio and live servers. It selects explicit IDs for 1,000 or 10,000 brushes. Timings
include JSON parsing, schema validation, group expansion and selection notification;
they exclude map construction, clearing the previous selection, socket transport and
GUI scheduling. Each request must finish with every requested brush selected.

On 2026-11-03, release binaries pinned to the same CPU produced the following
medians. Each value is the median of three runs of five samples, alternating the old
and new binaries, with no concurrent build or tests.

| Request | Before | After |
| --- | ---: | ---: |
| Select 1,000 explicit IDs | 22.059 ms | 0.411 ms |
| Select 10,000 explicit IDs | 2,734.302 ms | 4.853 ms |

Tool schemas are initialized once and borrowed for validation. Explicit selection
validates IDs together and expands closed groups once, then replaces the selection
with one notification. Selecting two members of the same group no longer toggles the
group off. Missing IDs preserve both object and face selections.

The editor's brush-to-face selection conversion also walks brushes once and uses
geometry face indices. Redundant source planes cannot leave invalid face references
in the selection.

## Freeform sculpting

Run the small-brush latency probe on the starting room:

```sh
cargo run --release -p editor_core --example sculpt_latency
```

It reports each dab, brush counts, final merging, and the number of dabs needed
for a 64-unit drag. Use `--release` when measuring interactive performance;
unoptimized geometry builds are substantially slower. A Freeform stroke uses
its effective radius, including the minimum imposed by the grid, for its dab
spacing. Surface sampling visits only the projection of the local dab region
on each face and reuses per-sample distance queries within a dab.

## Web measurements

- `examples/engine-checks/threejs/bench_web_exports.py` times repeated playtest exports
  and a single-brush move on demo maps; `bench_web_protocol.mjs` checks that applying the
  delta reconstructs the edited export exactly.
- `HERON_VIEWER_BENCH=1 node scripts/tests/web-playtest-viewer.test.mjs` times the
  playtest viewer's handling of live patches.
- `python3 scripts/tests/check_three_collision.py` tests the exported player's collision
  tree, up to 120,000 triangles under a 384 MiB Node heap.
- `scripts/serve_web_release.py --metrics` serves a release build with a startup probe
  (`document.documentElement.dataset.heronStartup`); `scripts/tests/serve_web_bundle_cache.py`
  checks the startup-archive cache.
- `scripts/tests/serve_web_editor_latency.py` opens generated maps
  (`web_playtest_fixture large:1000` or `large:10000`) in the real web editor and records
  playtest preparation time and main-thread timer gaps.

## Profiling the app

```sh
# Writes bench-1000/10000/50000.map, bench-animated.map and textured/ (2,000 textures):
HERON_BENCH_DIR="$PWD/bench-data" cargo test --release -p editor_core --features test-support \
    --test perf -- --ignored write_bench_maps

HERON_BRUSH_PROFILE=1 HERON_BRUSH_ORBIT=200 target/release/heron-brush --new-instance \
    bench-data/maps/bench-10000.map
```

`HERON_BRUSH_PROFILE=1` prints every two seconds the rolling average and p95 (ms) of
`frame` (CPU time of the frame, vsync excluded), `ui` (input, layout, panels), `2d` (the
flat views), `3d` (the 3D view, split into scene build and backend submission), `gpu`
(GPU time of the 3D view, where timer queries exist) and `paint` (egui tessellation and
painting). `HERON_BRUSH_ORBIT=<frames>` turns the camera round the map, prints an
`orbit` summary and quits. With the variable unset the hooks cost an `Option` check and
one relaxed atomic load (`editor_core::profile`).

For CPU profiles, build with `CARGO_PROFILE_RELEASE_STRIP=false
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` and attach a sampling profiler (`sample`,
`perf`, Instruments) to a benchmark selected with `HERON_BENCH_ONLY`.

## Representative numbers

Release builds on laptop-class hardware, medians in milliseconds. Single runs vary by
up to 2x with machine load; use them as orders of magnitude, not as targets.

| editor core | 1k | 10k | 50k |
|---|---:|---:|---:|
| `Document::open` | 5.8 | 56 | 247 |
| `save_as` | 2.5 | 26 | 133 |
| select all | 0.03 | 0.34 | 1.9 |
| undo a 10,000-brush move | – | 1.6 | 19 |
| validation (Issues) | 0.23 | 2.6 | 15 |
| export Godot / GLB | 27 / 25 | 290 / 282 | 1,626 / 1,773 |
| one flat view frame, nothing changed (egui) | – | 2.3 | 29 |
| one flat view frame while panning (egui) | – | ~28 | ~160–220 |

| editing on a 50k-brush map | ms |
|---|---:|
| 3D surface sync per drag step, 100 brushes selected | 1.7 |
| 3D surface sync per drag step, 10,000 selected | 16 |
| 3D surface sync after a selection click | 0.002 |
| rotate drag of 10,290 selected, 30 steps (model) | 421 |
| subtract 50 selected from the rest | 4.1 |
| 500 linked copies: edit one brush in one copy, then sync | 6.3 |
| 5,000 groups: Map tab frame | 0.37 |

Materials load from image headers: 2,000 / 10,000 textures take 55 / 148 ms with peak
RSS 57 / 73 MB, and the material browser's first frame 9 / 16 ms. Pixels are decoded on
demand; 200 newly visible textures take about 50–70 ms on eight threads. In the app, orbiting the camera with three flat views and the 3D view costs
about 6 ms of CPU per frame on 10k brushes and about 24 ms on 50k.

`map_load_probe`, `load` mode (parse and tree construction only):

| fixture | load | peak RSS |
|---|---:|---:|
| 1,000 boxes | 14 ms | 6 MiB |
| 10,000 boxes | 173 ms | 43 MiB |
| 50,000 boxes | 869 ms | 208 MiB |
| dragon ×40 (5,080 brushes) | 191 ms | 47 MiB |
| 10,000 groups + 10,000 point entities | 32 ms | 31 MiB |

Archives: mounting a 256 MiB PAK or ZIP grows RSS by under 2 MiB with the native
file-backed readers, against about 260 MiB with the whole-archive memory reader. Reading
20,000 tiny members costs about 280–300 ms file-backed against 135–160 ms from memory.

Web: the optimized release WASM is about 26.7 MB (9.2 MB gzip); startup data is one
content-addressed archive of 199 files, 8.2 MB (1.2 MB gzip), cached after the first
load. On a generated 10,000-brush map (120,000 triangles) the first playtest export
takes about 200 ms of main-thread preparation and about 4 s in the export worker.

## Known limits

- **Large drags.** Each step of a drag of thousands of brushes rolls back and rebuilds
  every selected brush: about 14–18 ms per step for 10,000 brushes, plus 16 ms of surfaces.
- **Flat views of very large maps.** Panning or zooming redraws the whole view picture
  each frame (~160–220 ms per view at 50k brushes). The unchanged frame is dominated by
  egui copying the kept mesh into the frame's vertex buffer and uploading it.
- **Forward-pass draws.** One draw and bind group per (chunk, material): about 8,000 on a
  50k-brush map. Bindless textures with indirect draws are the planned fix.
- **Full rebuilds.** Changes that touch everything (a material reload, colour
  preferences) rebuild every node's surfaces.
- **GL view and linked groups.** The fallback GL renderer uploads the whole mesh after
  every change; `sync_linked_groups` (the menu action) is still a full rebuild.
- **Undo history** keeps full copies of changed brushes, so memory grows with brushes
  changed times steps.
- **File-backed archives** are slower than memory-backed ones for many tiny members.
- **Web playtest.** Export preparation runs on the main thread (over 200 ms on 10k
  brushes); scene and collision rebuilds of large maps in the viewer take hundreds of
  milliseconds per edit.
- **Small maps parse slightly slower** with the streaming tokenizer than they did with a
  token vector; large maps parse faster.


## Group hierarchy loading and malformed input

Measured on Apple M4 on 2026-11-19 against `5d453df`, using `map_load_probe` in
release mode (`CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_LTO=false
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`). The comparison script alternated before,
after, after, before, with one warmup and 31 samples per process. macOS runs were
not pinned to a CPU. These timings cover parsing and tree construction, not texture
loading, filesystem reads or the first rendered frame.

| Input | Before (ms) | After (ms) |
|---|---:|---:|
| 4,096 groups, chains of 64 | 3.19 | 2.06 |
| 4,096 groups, chains of 128 | 3.89 | 2.07 |
| 4,096 independent groups | 1.46 | 1.51 |
| 1,000 boxes | 6.30 | 6.23 |
| Bundled dragon map | 1.58 | 1.54 |

Parent depths are resolved once per group without recursion. Stable ordering,
first-occurrence handling of duplicate IDs and the existing cycle-depth sentinel
are unchanged. Short timings outside the nested-group cases show only small
variation, not a demonstrated general speedup for all maps.

A 15,000-group chain caused a stack-overflow abort with a 1 MiB process stack before
the fix. Tree construction now limits nesting to 128 groups: an attachment that
would exceed the limit moves the entire subtree to the default layer and records a
warning. No group, brush or entity is discarded. The limit applies to map loading;
it is not a general limit imposed on every editing operation.

`map_load_resilience` covers deep chains and cycles on a 1 MiB thread stack through
both tree builders, cloning, saving, reopening and destruction, including geometry
and a named entity at the deepest point. It also checks the 128/129 boundary and
truncation/token mutations of a valid map. Unit tests compare depth results with the
previous algorithm for every four-node parent graph and exercise 50,000-node chains
and cycles. These are bounded regression cases, not a claim that every malformed
input is safe.


## Material discovery and first-frame stages

On Apple M4 (2026-11-19), reusing successful image headers when pairing normal maps
reduced discovery time for 512 albedos plus 512 normals from 16.58 ms to 10.27 ms
(38%). For 512 albedos without normals the medians were 5.11 and 5.20 ms. Fixtures
use copies of the bundled small clip PNG; these measurements do not represent large
pixel decodes, network storage or completely cold filesystem caches.

Build `material_load_probe` in `map_model` with the release settings above, preserving
a before and after executable. Run `scripts/tests/benchmark_material_loading.py
<before> <after> <results.json> --runs 31`. The script generates disposable fixtures
and alternates before/after/after/before serially. The collection loader retains its
existing bounded header-reader parallelism. The per-load header lookup is keyed by
full file path, including files superseded by higher-priority extensions; it is not
a persistent cache, so reloading a changed image reads its new dimensions.

A separate manual GPU diagnostic, `profile_first_frame_stages` in the `render_wgpu`
headless tests, measures document open, renderer initialization, scene sync, and the
first frame with pixel readback. Run with `--ignored --nocapture --test-threads=1`
and an isolated `XDG_CONFIG_HOME`. It uses a 640x480 view of the bundled dragon map.
Device creation is excluded, and asynchronous texture streaming need not have
settled when the first frame is read. There are no machine-dependent speed asserts.

The current **unoptimized development build** on Apple M4 reported:

| Stage | First renderer (ms) | Second (ms) | Third (ms) |
|---|---:|---:|---:|
| Open document and its material sources | 41.60 | 33.59 | 33.42 |
| Initialize renderer | 1041.18 | 215.61 | 214.33 |
| Sync geometry | 6.32 | 5.85 | 5.74 |
| First frame, GPU completion and readback | 43.33 | 26.41 | 23.22 |

These are stage diagnostics, not pure GPU timings or release performance claims.
The three renderers share one device, and driver caches may affect later rounds.
Renderer initialization is the next profiling target; it was not changed by the
material-header optimization.


## Renderer initialization: pass-specific helpers and shared MatCap pixels

The same Apple M4 development build was profiled further on 2026-11-19. With warm
driver caches, shader module creation took about 135–140 ms, pipeline creation
56–60 ms, and MatCap atlas baking 18–19 ms. Every shader previously received the
entire shared lighting/environment helper set and the surface-grid implementation,
even when it did not use them.

Each pass now includes only the helper groups it needs. WGSL function bodies and
frame/material layouts are unchanged. The immutable 384x128 RGBA8 MatCap atlas is
baked once per process (192 KiB retained); callers still receive independently owned
pixel buffers. This removes repeated CPU work without deferring shader setup into
the first frame or sharing device-bound GPU resources.

Four serial processes alternated before/after/after/before, creating three renderers
per process. Excluding each process's first renderer, the initialization median fell
from **209.55 to 139.52 ms (33.4%)**. The first-frame-plus-readback median was 25.14 ms
before and 24.31 ms after. The first after-process still encountered a cold driver
shader cache (729.60 ms), so this experiment does **not** establish a controlled
cold-start speedup or a release-build speedup. Device initialization remains outside
the measurement.

All 50 PNG references from display modes, effects and surface-grid checks matched
byte for byte. All 36 ordinary headless GPU tests passed; six manual benchmarks or
profiling tests remained ignored. The suite covers shadows, environment lighting,
reflections, transparency, normals, streaming, temporal filtering, MatCap and section
views. A unit test also compares cached MatCap output with the original bake and
checks that modifying a returned buffer cannot affect later callers.


## Additional renderer windows: device-scoped shader reuse

Live renderers created on the same thread and GPU device now share immutable shader
modules. Each renderer still owns its scene, pipelines, render targets, textures,
lighting and temporal state. A thread-local weak index neither keeps modules alive
after their last owner closes nor shares them across different devices. This targets
additional 3D and image-render windows, not the first renderer of a process.

Set `HERON_PROFILE_KEEP_FIRST_RENDERER=1` for `profile_first_frame_stages` to retain
the first renderer while measuring the next two. Four processes alternated
before/after/after/before on Apple M4 in the same development profile. The median for
additional renderer initialization fell from **133.67 to 53.99 ms (59.6%)**. The
first-frame-plus-readback median remained similar: 25.81 versus 25.33 ms. This does
not establish a cold-start improvement or release-build performance.

A separate `/usr/bin/time -l` run reported peak process RSS of 112.6 MiB before and
110.1 MiB after. This single sample is a sanity check, not evidence of a meaningful
memory reduction or a measurement of all GPU allocations. Shader modules are now
retained while their renderers remain open, instead of rebuilding them for each new
window; the weak-cache lifecycle test verifies release after the final owner closes.

All 37 ordinary headless GPU tests passed (six manual tests ignored). New tests
exercise two distinct devices, shared ownership and release, independent scene
replacement and resizing, closing one renderer while another remains usable, and
reopening after every renderer has closed.

## Deferred brush-primitive UV resolution

Material discovery now resolves pending brush-primitive UV matrices in one tree
walk. Previously it collected replacement brushes, then searched the tree again
for each ID and copied each replacement. Brushes whose pending materials are all
missing now return no replacement; their matrices remain available for later
material discovery. Geometry and UV conversion arithmetic are unchanged.

Reproduce the isolated traversal comparison with:

```sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_LTO=false \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
cargo run --locked --release -p map_model --example primitive_uv_probe
```

On Apple M4, seven measured rounds after one warmup alternated execution order.
Each round started from a fresh world clone outside the timer. The probe checks
that both algorithms produce identical serialized maps, including node IDs.

| Brushes with pending UV matrices | Repeated lookup | Single traversal |
| --- | ---: | ---: |
| 1,000 | 1.337 ms | 0.612 ms |
| 4,000 | 10.234 ms | 1.922 ms |
| 8,000 | 34.175 ms | 3.896 ms |

This measures UV resolution only, not total map opening, texture I/O, renderer
initialization or GPU time. Both paths use the same updated per-brush conversion;
the comparison isolates traversal and replacement costs when materials exist.
Regression tests cover missing and partially discovered materials, unchanged
geometry, UV coordinates, repeat resolution, and layers with nested groups and
brush entities. The document test also covers save/reopen, UV edits, undo and
subsequent texture discovery.

## Prepared UV sampling during surface construction

Surface construction prepares UV axes and safe scales once per face instead of
recomputing them for every triangle corner. Gradient colors reuse those corner UVs.
The projection is local to the current build, so edits cannot leave stale mappings.
Division, float conversion and offset addition retain the scalar path's order.

Run the isolated optimized sampling probe with the existing release configuration:

```sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_LTO=false \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
cargo run --locked --release -p map_model --example uv_projection_probe
```

Apple M4, 20,000 faces per run, alternating order, seven measured rounds after one
warmup (median milliseconds):

| Mapping | Corners per face | Scalar sampling | Prepared sampling |
| --- | ---: | ---: | ---: |
| Paraxial | 6 | 4.665 | 0.911 |
| Paraxial | 48 | 40.313 | 2.068 |
| Parallel | 6 | 0.472 | 0.239 |
| Parallel | 48 | 3.325 | 1.164 |

These are isolated UV sampling costs, not whole-scene build, map loading or frame
times. The probe includes projection preparation and checks identical accumulated
results. Tests compare UV bits across all six axis directions, a sloped normal,
rotations, offsets, skewed axes, negative, zero and tiny scales. A surface regression
test independently checks positions, UVs and gradient colors against scalar face
sampling for both mapping systems.

Validation passed: 295 model unit tests, eight surface tests and all 37 ordinary
headless GPU tests (six manual GPU benchmarks ignored). All 50 saved GPU reference
PNGs matched byte for byte after the change.
