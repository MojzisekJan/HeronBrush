//! The editor's performance benchmarks. See `docs/performance.md`.
//!
//! Ignored by default; run in release, one at a time so the timings do not fight:
//!
//! ```text
//! cargo test --release -p editor_core --features test-support --test perf -- --ignored --test-threads=1 --nocapture
//! ```
//!
//! `HERON_BENCH_SIZES=1000,10000` narrows the map sizes (default `1000,10000,50000`),
//! `HERON_BENCH_RUNS` sets the repetitions (default 5), `HERON_BENCH_TEXTURES` the
//! texture counts (default `2000,5000`). Each test prints a Markdown table on stdout.

#![allow(missing_docs)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use editor_core::bench_support::{self as bench, MapSpec, Scratch, Stats};
use editor_core::canvas::{Canvas, FillRule, LineCap, LineJoin, TextExtents};
use editor_core::document::Document;
use editor_core::view_filters::ViewFilters;
use editor_core::world::WorldBounds;
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

fn sizes() -> Vec<usize> {
    std::env::var("HERON_BENCH_SIZES")
        .ok()
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![1_000, 10_000, 50_000])
}

fn runs() -> usize {
    std::env::var("HERON_BENCH_RUNS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(5)
}

/// Fewer runs for the slow cases on the big maps.
fn runs_for(brushes: usize) -> usize {
    if brushes >= 50_000 {
        runs().min(3)
    } else {
        runs()
    }
}

/// Writes the synthetic map for `spec` into `dir` and returns its path.
fn write_map(dir: &Path, name: &str, spec: &MapSpec) -> PathBuf {
    let path = dir.join(format!("{name}.map"));
    std::fs::write(&path, bench::synthetic_map(spec)).unwrap();
    path
}

fn open(path: &Path) -> Document {
    Document::open(path, WorldBounds::default()).expect("the synthetic map opens")
}

/// A canvas that draws nothing and counts what it was asked to draw, so the 2D view's
/// own work is measured without any backend.
#[derive(Default)]
struct Counting {
    segments: usize,
    strokes: usize,
    texts: usize,
}

impl Canvas for Counting {
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: f64, _: f64) {}
    fn scale(&mut self, _: f64, _: f64) {}
    fn new_path(&mut self) {}
    fn new_sub_path(&mut self) {}
    fn move_to(&mut self, _: f64, _: f64) {}
    fn line_to(&mut self, x: f64, _: f64) {
        self.segments += 1;
        std::hint::black_box(x);
    }
    fn arc(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64) {}
    fn arc_negative(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64) {}
    fn rectangle(&mut self, _: f64, _: f64, _: f64, _: f64) {
        self.segments += 4;
    }
    fn close_path(&mut self) {}
    fn set_source_rgba(&mut self, _: f64, _: f64, _: f64, _: f64) {}
    fn set_line_width(&mut self, _: f64) {}
    fn set_dash(&mut self, _: &[f64], _: f64) {}
    fn set_line_cap(&mut self, _: LineCap) {}
    fn set_line_join(&mut self, _: LineJoin) {}
    fn set_fill_rule(&mut self, _: FillRule) {}
    fn stroke_preserve(&mut self) {
        self.strokes += 1;
    }
    fn fill_preserve(&mut self) {}
    fn paint(&mut self) {}
    fn clip(&mut self) {}
    fn paint_tiled_image(&mut self, _: &str, _: u32, _: u32, _: &[u8], _: (f64, f64), _: f64) {}
    fn set_font_bold(&mut self, _: bool) {}
    fn show_text(&mut self, _: &str, _: f64) {
        self.texts += 1;
    }
    fn show_text_outlined(&mut self, _: &str, _: f64, _: [f64; 4], _: f64) {
        self.texts += 1;
    }
    fn text_extents(&mut self, text: &str, size: f64) -> TextExtents {
        TextExtents {
            width: text.len() as f64 * size * 0.5,
            height: size,
            x_advance: text.len() as f64 * size * 0.5,
            line_height: size * 1.2,
            baseline: size,
            ..Default::default()
        }
    }
}

/// About `fraction` of the brushes, the ones nearest the middle of the map, selected.
fn select_middle(document: &mut Document, count: usize) {
    let centre = document.bounds().map(|b| b.center()).unwrap_or_default();
    let mut brushes: Vec<(f64, map_model::node::NodeId)> = document
        .selectable_brushes()
        .into_iter()
        .map(|(id, b)| {
            let d = b.bounds().center() - centre;
            (geometry_math::vec::dot(d, d), id)
        })
        .collect();
    brushes.sort_by(|a, b| a.0.total_cmp(&b.0));
    document.clear_selection();
    for (_, id) in brushes.into_iter().take(count) {
        document.toggle_selected(id);
    }
}

#[test]
#[ignore]
fn perf_load_save() {
    let scratch = Scratch::new("load");
    let mut rows: Vec<Stats> = Vec::new();
    for n in sizes() {
        let runs = runs_for(n);
        let start = Instant::now();
        let text = bench::synthetic_map(&MapSpec::brushes(n));
        eprintln!(
            "{n}: generated {} KB of map text in {:.0} ms",
            text.len() / 1024,
            bench::ms(start.elapsed())
        );
        let path = scratch.0.join(format!("m{n}.map"));
        std::fs::write(&path, &text).unwrap();
        rows.push(bench::measure(
            &format!("{n} brushes: read_map (parse only)"),
            runs,
            || (),
            |_| map_model::io::map_reader::read_map(&text, None).unwrap(),
        ));
        let map = map_model::io::map_reader::read_map(&text, None).unwrap();
        let world = Bbox3d::from_scalars(-16384.0, 16384.0);
        rows.push(bench::measure(
            &format!("{n} brushes: build_tree"),
            runs,
            || (),
            |_| map_model::node::build_tree(&map, &world),
        ));
        rows.push(bench::measure(
            &format!("{n} brushes: Document::open"),
            runs,
            || (),
            |_| open(&path),
        ));
        let mut document = open(&path);
        eprintln!("{n}: {} brushes in the document", document.brush_count());
        rows.push(bench::measure(
            &format!("{n} brushes: to_map_text"),
            runs,
            || (),
            |_| document.to_map_text(),
        ));
        let saved = scratch.0.join(format!("m{n}-saved.map"));
        rows.push(bench::measure(
            &format!("{n} brushes: save_as"),
            runs,
            || (),
            |_| document.save_as(&saved).unwrap(),
        ));
        rows.push(bench::measure(
            &format!("{n} brushes: asset preview 256px"),
            runs,
            || (),
            |_| editor_core::assets::preview(&text, 256),
        ));
    }
    // The thumbnail size the asset browser actually draws, on a prefab-sized map.
    let prefab = bench::synthetic_map(&MapSpec::brushes(200));
    rows.push(bench::measure(
        "200-brush prefab: asset preview 128px",
        runs(),
        || (),
        |_| editor_core::assets::preview(&prefab, 128),
    ));
    bench::print_table("Load and save", &rows);
}

#[test]
#[ignore]
fn perf_selection_and_transforms() {
    let scratch = Scratch::new("edit");
    let mut rows: Vec<Stats> = Vec::new();
    let filters = ViewFilters::default();
    for n in sizes() {
        let runs = runs_for(n);
        let path = write_map(&scratch.0, &format!("m{n}"), &MapSpec::brushes(n));
        let mut document = open(&path);
        rows.push(bench::measure_on(
            &format!("{n}: select all"),
            runs,
            &mut document,
            |d| d.clear_selection(),
            |d| d.select_all(),
        ));
        rows.push(bench::measure_on(
            &format!("{n}: select none"),
            runs,
            &mut document,
            |d| d.select_all(),
            |d| d.clear_selection(),
        ));

        for selected in [100usize, 1_000, 10_000] {
            if selected > n {
                continue;
            }
            select_middle(&mut document, selected);
            // A drag of 60 pointer moves, each nudging by a grid step, as one undo step;
            // the model only, then with what each frame redraws (the flat views' edges
            // and the 3D view's surfaces) on top.
            rows.push(bench::measure(
                &format!("{n}: move {selected} selected, 60-step drag (model)"),
                runs,
                || (),
                |_| {
                    document.begin_transaction();
                    for _ in 0..60 {
                        document.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
                    }
                    document.commit_transaction();
                },
            ));
            rows.push(bench::measure(
                &format!("{n}: move {selected} selected, 60-step drag + redraws"),
                runs.min(3),
                || (),
                |_| {
                    document.begin_transaction();
                    for _ in 0..60 {
                        document.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
                        std::hint::black_box(document.edges().len());
                        std::hint::black_box(document.preview_mesh(&filters).triangles.len());
                    }
                    document.commit_transaction();
                },
            ));
            rows.push(bench::measure(
                &format!("{n}: undo the {selected}-brush move"),
                runs,
                || (),
                |_| document.undo(),
            ));
            rows.push(bench::measure(
                &format!("{n}: redo the {selected}-brush move"),
                runs,
                || (),
                |_| document.redo(),
            ));
            let centre = document.selection_bounds().unwrap().center();
            rows.push(bench::measure(
                &format!("{n}: rotate {selected} selected, 60-step drag (model)"),
                runs,
                || (),
                |_| {
                    document.begin_transaction();
                    for step in 1..=60 {
                        document.rollback_transaction();
                        document.rotate_selection_about(
                            centre,
                            Vec3d::new(0.0, 0.0, 1.0),
                            step as f64 * 1.5,
                        );
                    }
                    document.commit_transaction();
                },
            ));
            let start = document.selection_bounds().unwrap();
            rows.push(bench::measure(
                &format!("{n}: scale {selected} selected, 60-step drag (model)"),
                runs,
                || (),
                |_| {
                    document.begin_transaction();
                    for step in 1..=60 {
                        document.rollback_transaction();
                        let grown = Bbox3d::new(
                            start.min,
                            start.max + Vec3d::new(step as f64 * 4.0, step as f64 * 2.0, 0.0),
                        );
                        document.transform_selection(&geometry_math::mat_ext::scale_bbox_matrix(
                            &start, &grown,
                        ));
                    }
                    document.commit_transaction();
                },
            ));
        }
        document.clear_selection();
        rows.push(bench::measure_on(
            &format!("{n}: undo+redo a single small edit"),
            runs,
            &mut document,
            |document| {
                select_middle(document, 1);
                document.translate_selection(Vec3d::new(0.0, 8.0, 0.0));
            },
            |document| {
                document.undo();
                document.redo();
            },
        ));
    }
    bench::print_table("Selection and transforms", &rows);
}

#[test]
#[ignore]
fn perf_geometry_tools() {
    let scratch = Scratch::new("geometry");
    let mut rows: Vec<Stats> = Vec::new();
    for n in sizes() {
        let runs = runs_for(n);
        let path = write_map(&scratch.0, &format!("m{n}"), &MapSpec::brushes(n));
        let base = open(&path);
        let text = base.to_map_text();
        drop(base);
        let fresh = || {
            let p = scratch.0.join("fresh.map");
            std::fs::write(&p, &text).unwrap();
            open(&p)
        };
        let mut document = fresh();
        select_middle(&mut document, 50);
        let count = document.brush_count();
        let bounds = document.selection_bounds().unwrap();
        let mut clip = editor_core::clip_tool::ClipTool::new();
        let c = bounds.center();
        clip.set_face([
            c,
            c + Vec3d::new(0.0, 1.0, 0.0),
            c + Vec3d::new(0.3, 0.0, 1.0),
        ]);
        rows.push(bench::measure_on(
            &format!("{n}: clip 50 selected brushes"),
            runs,
            &mut document,
            |d| select_middle(d, 50),
            |document| {
                let r = document.run_as_one_edit(|d| d.clip_selection(&clip));
                assert!(r > 0, "the clip cut something");
                document.undo();
                r
            },
        ));
        rows.push(bench::measure_on(
            &format!("{n}: hollow 50 selected brushes"),
            runs,
            &mut document,
            |d| select_middle(d, 50),
            |document| {
                let r = document.run_as_one_edit(|d| d.hollow_selection(8.0));
                document.undo();
                r
            },
        ));
        rows.push(bench::measure_on(
            &format!("{n}: subtract 5 selected from the rest"),
            runs,
            &mut document,
            |d| select_middle(d, 5),
            |document| {
                let r = document.run_as_one_edit(|d| d.subtract_selection());
                document.undo();
                r
            },
        ));
        rows.push(bench::measure_on(
            &format!("{n}: merge 20 selected"),
            runs,
            &mut document,
            |d| select_middle(d, 20),
            |document| {
                let r = document.run_as_one_edit(|d| d.merge_selection());
                document.undo();
                r
            },
        ));

        assert_eq!(document.brush_count(), count, "every edit above was undone");
        // A big brush: a 64-sided cylinder, its top ring of vertices dragged 60 times.
        document.clear_selection();
        let big = map_model::shapes::create_cylinder(
            &document.world_bounds(),
            &Bbox3d::new(
                Vec3d::new(0.0, 0.0, 4096.0),
                Vec3d::new(1024.0, 1024.0, 4608.0),
            ),
            map_model::shapes::CircleShape::EdgeAligned { num_sides: 64 },
            geometry_math::util::Axis::Z,
            "bench/m00001",
        )
        .unwrap();
        let top: Vec<Vec3d> = big
            .vertex_positions()
            .into_iter()
            .filter(|v| v.z() > 4600.0)
            .take(8)
            .collect();
        document.add_brushes(vec![big]);
        let id = *document
            .selection_ids()
            .iter()
            .next()
            .expect("the new brush is selected");
        document.select_only(id);
        rows.push(bench::measure(
            &format!("{n}: vertex drag on a 64-sided cylinder (8 vertices, 60 steps)"),
            runs,
            || (),
            |_| {
                document.begin_transaction();
                let mut at = top.clone();
                for _ in 0..60 {
                    if document.move_handles(&at, Vec3d::new(0.0, 0.0, 1.0)) {
                        for v in &mut at {
                            *v = *v + Vec3d::new(0.0, 0.0, 1.0);
                        }
                    }
                }
                document.commit_transaction();
                document.undo();
            },
        ));
    }
    bench::print_table("Geometry tools", &rows);
}

#[test]
#[ignore]
fn perf_scene_build_and_views() {
    let scratch = Scratch::new("views");
    let mut rows: Vec<Stats> = Vec::new();
    let filters = ViewFilters::default();
    for n in sizes() {
        let runs = runs_for(n);
        let path = write_map(&scratch.0, &format!("m{n}"), &MapSpec::brushes(n));
        let mut document = open(&path);
        let bump = |d: &mut Document| d.mark_changed();
        rows.push(bench::measure_on(
            &format!("{n}: 3D scene surfaces (preview_mesh, cold)"),
            runs,
            &mut document,
            |d| bump(d),
            |d| d.preview_mesh(&filters).triangles.len(),
        ));
        let triangles = document.preview_mesh(&filters).triangles.len();
        eprintln!("{n}: {triangles} triangles");
        select_middle(&mut document, 100);
        rows.push(bench::measure_on(
            &format!("{n}: 3D scene surfaces after a selection change"),
            runs,
            &mut document,
            |d| select_middle(d, 100),
            |d| d.preview_mesh(&filters).triangles.len(),
        ));
        rows.push(bench::measure_on(
            &format!("{n}: flat view edge cache (cold)"),
            runs,
            &mut document,
            |d| bump(d),
            |d| d.edges().len(),
        ));
        let (_, viewport) = ((), (1200.0, 800.0));
        for plane in [
            editor_core::camera2d::ViewPlane::Top,
            editor_core::camera2d::ViewPlane::Front,
        ] {
            let bounds = WorldBounds::default();
            let mut state = editor_core::view2d::ViewportState {
                camera: editor_core::camera2d::Camera2d::new(plane, bounds.half_extent()),
                cursor: None,
                bounds,
                framed_generation: None,
                creating: None,
                creating_elsewhere: None,
                lasso: None,
                scaling: None,
                rotating: false,
                moving: false,
                dragging_handle: None,
                entity_drop: None,
            };
            editor_core::view2d::frame_new_document(&mut state, &document, viewport);
            let grid = editor_core::grid::Grid::new(4);
            let mut segments = 0;
            rows.push(bench::measure(
                &format!(
                    "{n}: flat view draw, {} (whole map framed, warm caches)",
                    plane.name()
                ),
                runs,
                || (),
                |_| {
                    let mut canvas = Counting::default();
                    editor_core::view2d::draw(
                        &state,
                        &grid,
                        &document,
                        &filters,
                        &mut canvas,
                        viewport,
                    );
                    editor_core::view2d::draw_selection_bounds(
                        &state,
                        &document,
                        &mut canvas,
                        viewport,
                    );
                    segments = canvas.segments;
                },
            ));
            eprintln!("{n}: {} view asks for {segments} segments", plane.name());
        }
        rows.push(bench::measure(
            &format!("{n}: overlay (selection outline, 100 selected)"),
            runs,
            || (),
            |_| {
                let mut guides = 0;
                for brush in document.selected_brushes() {
                    guides += filters.brush_edges(brush).len();
                }
                guides
            },
        ));
        rows.push(bench::measure(
            &format!("{n}: validation (Issues)"),
            runs_for(n).min(3),
            || (),
            |_| editor_core::validation::validate(&document).len(),
        ));
        for target in [
            editor_core::export::Target::Godot,
            editor_core::export::Target::Glb,
        ] {
            let out = scratch.0.join(format!("export-{target:?}"));
            let options = editor_core::export::Options {
                target,
                ..Default::default()
            };
            rows.push(bench::measure(
                &format!("{n}: export {target:?}"),
                runs_for(n).min(3),
                || (),
                |_| editor_core::export::export(&document, &out, &options).unwrap(),
            ));
            let _ = std::fs::remove_dir_all(&out);
        }
    }
    bench::print_table("3D scene build, flat views, validation, export", &rows);
}

#[test]
#[ignore]
fn perf_animation() {
    let scratch = Scratch::new("animation");
    let mut rows: Vec<Stats> = Vec::new();
    for n in [1_000usize, 10_000] {
        let runs = runs();
        let spec = MapSpec {
            brushes: n,
            materials: 64,
            animated: 200,
            seed: 7,
        };
        let path = write_map(&scratch.0, &format!("a{n}"), &spec);
        let document = open(&path);
        let mut filters = ViewFilters::default();
        rows.push(bench::measure(
            &format!("{n} brushes, 200 func_animate: motions (per document change)"),
            runs,
            || (),
            |_| document.preview_motions().len(),
        ));
        let mesh = document.preview_mesh(&filters);
        eprintln!(
            "{n}: {} triangles, {} moving ranges",
            mesh.triangles.len(),
            mesh.motions.len()
        );
        rows.push(bench::measure(
            &format!("{n} brushes, 200 func_animate: posed triangles per frame (Mesh::at)"),
            runs,
            || (),
            |_| mesh.at(1.25).len(),
        ));
        rows.push(bench::measure(
            &format!("{n} brushes, 200 func_animate: scrubbed frame (Mesh::at_frame)"),
            runs,
            || (),
            |_| mesh.at_frame(1.25).len(),
        ));
        filters.preview_motion = true;
        rows.push(bench::measure(
            &format!("{n} brushes, 200 func_animate: flat view wireframe per frame"),
            runs,
            || (),
            |_| document.animation_wireframe(&filters).plain.len(),
        ));
        // One entity's pose and its motion path.
        let entity = map_model::node::entities_with_ids(document.tree_world())
            .into_iter()
            .find(|(_, e)| e.classname() == Some("func_animate"))
            .map(|(id, _)| id)
            .unwrap();
        rows.push(bench::measure(
            &format!("{n} brushes: read one pose"),
            runs,
            || (),
            |_| editor_core::animation_pose::Pose::read(&document, entity, 1.0).unwrap(),
        ));
        let pose = editor_core::animation_pose::Pose::read(&document, entity, 1.0).unwrap();
        rows.push(bench::measure(
            &format!("{n} brushes: build one motion path"),
            runs,
            || (),
            |_| editor_core::animation_path::Path::new(&pose),
        ));
        rows.push(bench::measure(
            &format!("{n} brushes: poses of all 200 entities"),
            runs,
            || (),
            |_| {
                map_model::node::entities_with_ids(document.tree_world())
                    .into_iter()
                    .filter(|(_, e)| e.classname() == Some("func_animate"))
                    .filter_map(|(id, _)| {
                        editor_core::animation_pose::Pose::read(&document, id, 1.0).ok()
                    })
                    .map(|p| p.centre().x())
                    .sum::<f64>()
            },
        ));
    }
    bench::print_table("Animation preview", &rows);
}

#[test]
#[ignore]
fn perf_materials() {
    let counts: Vec<usize> = std::env::var("HERON_BENCH_TEXTURES")
        .ok()
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![2_000, 5_000]);
    let mut rows: Vec<Stats> = Vec::new();
    for count in counts {
        let scratch = Scratch::new(&format!("textures{count}"));
        let start = Instant::now();
        let bytes = bench::write_textures(&scratch.0, count, 4, 3).unwrap();
        eprintln!(
            "{count}: wrote {:.1} MB of PNG in {:.1} s",
            bytes as f64 / 1e6,
            start.elapsed().as_secs_f64()
        );
        let rss_before = bench::current_rss();
        let collection = bench::measure(
            &format!("{count} textures + 4 at 2048px: MaterialCollection::load_directory"),
            1,
            || (),
            |_| map_model::material::MaterialCollection::load_directory(&scratch.0).unwrap(),
        );
        rows.push(collection);
        let mut document = Document::new(WorldBounds::default());
        rows.push(bench::measure(
            &format!("{count} textures: Document::load_materials"),
            1,
            || (),
            |_| document.load_materials(&scratch.0).unwrap(),
        ));
        let pixels: usize = document
            .materials()
            .materials()
            .iter()
            .map(|m| m.width as usize * m.height as usize * 4)
            .sum();
        let rss_after = bench::current_rss();
        println!(
            "\n{count} textures: {} materials, {:.0} MB of RGBA pixels; RSS {:.0} MB -> {:.0} MB, peak {:.0} MB",
            document.materials().materials().len(),
            pixels as f64 / 1e6,
            rss_before as f64 / 1e6,
            rss_after as f64 / 1e6,
            bench::peak_rss() as f64 / 1e6
        );
        // The pixels are decoded on demand. What a view that comes to show 200
        // of them pays: on one thread, and spread over eight as the renderer's workers
        // and the browser's do (the decodes are dropped again, as streaming does once the
        // mips are uploaded).
        let sample: Vec<map_model::material::Material> = document
            .materials()
            .materials()
            .iter()
            .filter(|m| m.is_lazy())
            .step_by((count / 200).max(1))
            .take(200)
            .cloned()
            .collect();
        rows.push(bench::measure(
            &format!("{count} textures: decode 200 on demand, 1 thread"),
            1,
            || (),
            |_| {
                sample
                    .iter()
                    .map(|m| m.pixels().map_or(0, |p| p.len()))
                    .sum::<usize>()
            },
        ));
        let fresh = map_model::material::MaterialCollection::load_directory(&scratch.0).unwrap();
        let sample: Vec<map_model::material::Material> = fresh
            .materials()
            .iter()
            .filter(|m| m.is_lazy())
            .step_by((count / 200).max(1))
            .take(200)
            .cloned()
            .collect();
        rows.push(bench::measure(
            &format!("{count} textures: decode 200 on demand, 8 threads"),
            1,
            || (),
            |_| {
                std::thread::scope(|scope| {
                    let parts: Vec<_> = sample
                        .chunks(25)
                        .map(|part| {
                            scope.spawn(move || {
                                part.iter()
                                    .map(|m| m.pixels().map_or(0, |p| p.len()))
                                    .sum::<usize>()
                            })
                        })
                        .collect();
                    parts.into_iter().map(|p| p.join().unwrap()).sum::<usize>()
                })
            },
        ));
        drop(fresh);
        println!(
            "{count} textures: RSS after the on-demand decodes {:.0} MB",
            bench::current_rss() as f64 / 1e6
        );
        // A map wearing all of them: what the 3D view's surfaces cost with that many
        // materials (the batches are per material).
        let map = scratch.0.join("textured.map");
        std::fs::write(
            &map,
            bench::synthetic_map(&MapSpec {
                brushes: 10_000,
                materials: count,
                animated: 0,
                seed: 5,
            }),
        )
        .unwrap();
        let filters = ViewFilters::default();
        let mut textured = open(&map);
        textured.load_materials(&scratch.0).unwrap();
        rows.push(bench::measure_on(
            &format!("10k brushes over {count} materials: preview_mesh"),
            runs(),
            &mut textured,
            |d| d.mark_changed(),
            |d| d.preview_mesh(&filters).triangles.len(),
        ));
        drop(textured);
        drop(document);
    }
    bench::print_table("Materials", &rows);
}

/// Not a benchmark: writes the generated maps (and a texture folder the textured map
/// wears) into `$HERON_BENCH_DIR/maps`, for opening in the editor with
/// `HERON_BRUSH_PROFILE=1 HERON_BRUSH_ORBIT=300`.
#[test]
#[ignore]
fn write_bench_maps() {
    let root = std::env::var_os("HERON_BENCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("maps");
    std::fs::create_dir_all(&root).unwrap();
    for n in sizes() {
        write_map(&root, &format!("bench-{n}"), &MapSpec::brushes(n));
    }
    write_map(
        &root,
        "bench-animated",
        &MapSpec {
            brushes: 10_000,
            materials: 64,
            animated: 200,
            seed: 7,
        },
    );
    // A map over 2,000 materials, with the textures in `textures/` beside it, which is
    // where a document looks for a project's materials.
    let textured = root.join("textured");
    std::fs::create_dir_all(&textured).unwrap();
    write_map(
        &textured,
        "bench-textured",
        &MapSpec {
            brushes: 10_000,
            materials: 2_000,
            animated: 0,
            seed: 5,
        },
    );
    bench::write_textures(&textured.join("textures"), 2_000, 4, 3).unwrap();
    eprintln!("wrote {}", root.display());
}

/// A retained renderer that keeps nothing: counts what a sync sends, so the tracker's own
/// cost (and the document's surfaces under it) is what is measured.
#[derive(Default)]
struct CountingSink {
    triangles: usize,
    calls: usize,
}

impl editor_core::render_scene::SceneSink for CountingSink {
    fn set_materials(&mut self, _: &[map_model::material::Material]) {}
    fn clear(&mut self) {
        self.calls += 1;
    }
    fn remove(&mut self, _: editor_core::render_scene::ObjectKey) {
        self.calls += 1;
    }
    fn upsert(
        &mut self,
        _: editor_core::render_scene::ObjectKey,
        triangles: &[render::Triangle],
        _: editor_core::render_scene::ObjectState,
    ) {
        self.calls += 1;
        self.triangles += triangles.len();
    }
    fn set_state(
        &mut self,
        _: editor_core::render_scene::ObjectKey,
        _: editor_core::render_scene::ObjectState,
    ) {
        self.calls += 1;
    }
}

/// What a frame of an edit costs the 3D view's surfaces -- the document's
/// preview mesh and the retained renderer's sync -- on top of the edit itself.
#[test]
#[ignore]
fn perf_surfaces_while_editing() {
    use editor_core::render_scene::{Pose, SceneTracker};
    let scratch = Scratch::new("surfaces");
    let mut rows: Vec<Stats> = Vec::new();
    let filters = ViewFilters::default();
    let sizes: Vec<usize> = std::env::var("HERON_BENCH_SIZES")
        .ok()
        .map(|_| sizes())
        .unwrap_or_else(|| vec![10_000, 50_000]);
    for n in sizes {
        let runs = runs_for(n).min(3);
        let path = write_map(&scratch.0, &format!("m{n}"), &MapSpec::brushes(n));
        // A world four times as wide as the map, so turning thousands of brushes keeps
        // them inside.
        let mut document = Document::open(&path, WorldBounds::new(65_536.0)).unwrap();
        let mut tracker = SceneTracker::new();
        let mut sink = CountingSink::default();
        tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
        rows.push(bench::measure_on(
            &format!("{n}: tracker sync after a selection click"),
            runs,
            &mut document,
            |d| select_middle(d, 1),
            |d| tracker.sync(d, &filters, None, Pose::Rest, &mut sink),
        ));
        for selected in [100usize, 10_000] {
            if selected > n {
                continue;
            }
            select_middle(&mut document, selected);
            tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
            let centre = document.selection_bounds().unwrap().center();
            // Each step is what a frame of a rotate drag does: roll back, turn, then what
            // the 3D view asks for.
            for (what, surfaces) in [
                ("model only", 0),
                ("+ tracker sync (wgpu)", 1),
                ("+ preview_mesh (GL)", 2),
            ] {
                rows.push(bench::measure(
                    &format!("{n}: rotate {selected} selected, 10 steps, {what}"),
                    runs,
                    || (),
                    |_| {
                        document.begin_transaction();
                        for step in 1..=10 {
                            document.rollback_transaction();
                            assert!(document.rotate_selection_about(
                                centre,
                                Vec3d::new(0.0, 0.0, 1.0),
                                step as f64 * 1.7
                            ));
                            match surfaces {
                                1 => {
                                    tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
                                }
                                2 => {
                                    std::hint::black_box(
                                        document.preview_mesh(&filters).triangles.len(),
                                    );
                                }
                                _ => {}
                            }
                        }
                        document.commit_transaction();
                        document.undo();
                        if surfaces == 1 {
                            tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
                        }
                    },
                ));
            }
        }
        document.clear_selection();
        select_middle(&mut document, 1);
        document.translate_selection(Vec3d::new(0.0, 8.0, 0.0));
        tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
        rows.push(bench::measure_on(
            &format!("{n}: undo+redo a 1-brush move, tracker sync each"),
            runs,
            &mut document,
            |_| (),
            |d| {
                d.undo();
                tracker.sync(d, &filters, None, Pose::Rest, &mut sink);
                d.redo();
                tracker.sync(d, &filters, None, Pose::Rest, &mut sink)
            },
        ));
        eprintln!(
            "{n}: sink saw {} calls, {} triangles",
            sink.calls, sink.triangles
        );
    }

    // 20,000 point entities: moving a thousand of them.
    let path = scratch.0.join("entities.map");
    std::fs::write(&path, bench::point_entity_map(20_000, 5_000)).unwrap();
    let mut document = open(&path);
    let mut tracker = SceneTracker::new();
    let mut sink = CountingSink::default();
    tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
    let entities: Vec<map_model::node::NodeId> =
        map_model::node::point_entities(document.tree_world())
            .into_iter()
            .map(|(id, _)| id)
            .take(1_000)
            .collect();
    document.clear_selection();
    for id in &entities {
        document.toggle_selected(*id);
    }
    for (what, surfaces) in [("model only", false), ("+ tracker sync", true)] {
        rows.push(bench::measure(
            &format!("20k entities: move 1,000, 10 steps, {what}"),
            runs().min(3),
            || (),
            |_| {
                document.begin_transaction();
                for _ in 0..10 {
                    document.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
                    if surfaces {
                        tracker.sync(&document, &filters, None, Pose::Rest, &mut sink);
                    }
                }
                document.commit_transaction();
            },
        ));
    }
    bench::print_table("3D surfaces while editing", &rows);
}
