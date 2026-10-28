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
  `build_tree` path). `scripts/tests/generate_map_load_fixtures.py <dir>` writes the five
  fixtures; `scripts/tests/benchmark_map_loading.py <before> <after> <fixtures> <out.json>
  --cpu N` alternates two prebuilt probes, one process at a time, pinned to one CPU
  (Linux, `taskset`).
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
