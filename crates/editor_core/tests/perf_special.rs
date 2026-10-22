//! The special-case performance benchmarks (`docs/performance.md`).
//! Many groups, nesting and linked groups; transforms of very large selections with and
//! without alignment lock; handle drags, clip and subtract on big selections; many point
//! entities, many layers, one huge brush; undo history growth.
//!
//! ```text
//! cargo test --release -p editor_core --features test-support --test perf_special -- --ignored --test-threads=1 --nocapture
//! ```
//!
//! `HERON_BENCH_RUNS` sets the repetitions (default 5); `HERON_BENCH_ONLY` narrows to
//! the benchmarks whose name contains it.

#![allow(missing_docs)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use editor_core::bench_support::{self as bench, GroupSpec, MapSpec, Scratch, Stats};
use editor_core::document::Document;
use editor_core::view_filters::ViewFilters;
use editor_core::world::WorldBounds;
use geometry_math::bbox::Bbox3d;
use geometry_math::util::Axis;
use geometry_math::vec::Vec3d;
use map_model::node::NodeId;

fn runs() -> usize {
    std::env::var("HERON_BENCH_RUNS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(5)
}

fn write(dir: &Path, name: &str, text: String) -> PathBuf {
    let path = dir.join(format!("{name}.map"));
    std::fs::write(&path, text).unwrap();
    path
}

fn open(path: &Path) -> Document {
    Document::open(path, WorldBounds::default()).expect("the synthetic map opens")
}

/// The node id of the group whose `_tb_id` is `tb_id`.
fn group_node(document: &Document, tb_id: u64) -> NodeId {
    map_model::node::groups_with_ids(document.tree_world())
        .into_iter()
        .find(|(_, g)| g.id == tb_id.to_string())
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("group {tb_id} is in the map"))
}

/// The first brush inside the group whose `_tb_id` is `tb_id`, however deep.
fn member_of(document: &Document, tb_id: u64) -> NodeId {
    let group = group_node(document, tb_id);
    map_model::node::contents_of(document.tree_world(), group)[0]
}

/// The deepest brush of a chain: the last of the chain's groups holds it.
fn deepest_member(document: &Document, spec: &GroupSpec) -> NodeId {
    member_of(document, bench::FIRST_CHAIN_ID + spec.depth as u64 - 1)
}

fn select_middle(document: &mut Document, count: usize) {
    let centre = document.bounds().map(|b| b.center()).unwrap_or_default();
    let mut brushes: Vec<(f64, NodeId)> = document
        .selectable_brushes()
        .into_iter()
        .map(|(id, b)| {
            let d = b.bounds().center() - centre;
            (geometry_math::vec::dot(d, d), id)
        })
        .collect();
    brushes.sort_by(|a, b| a.0.total_cmp(&b.0));
    document.clear_selection();
    let caught: Vec<NodeId> = brushes.into_iter().take(count).map(|(_, id)| id).collect();
    let empty = document.selection_snapshot();
    document.preview_marquee(&empty, &caught, false);
}

fn set_alignment_lock(on: bool) {
    editor_core::preferences::edit(|p| p.alignment_lock = on);
}

#[test]
#[ignore]
fn perf_groups() {
    let scratch = Scratch::new("groups");
    let mut rows: Vec<Stats> = Vec::new();
    let runs = runs();
    let spec = GroupSpec::default();
    let path = write(&scratch.0, "groups", bench::grouped_map(&spec));
    let start = Instant::now();
    // A world four times as wide as the map, so turning the big group (a strip across
    // the whole map) keeps it inside.
    let mut document = Document::open(&path, WorldBounds::new(65_536.0)).unwrap();
    eprintln!(
        "groups map: {} brushes, {} groups, opened in {:.0} ms",
        document.brush_count(),
        document.groups().len(),
        bench::ms(start.elapsed())
    );
    rows.push(bench::measure(
        "5k groups: open the map",
        runs.min(3),
        || (),
        |_| open(&path),
    ));
    rows.push(bench::measure(
        "5k groups: groups() (the Map tab's list)",
        runs,
        || (),
        |_| document.groups().len(),
    ));
    rows.push(bench::measure_on(
        "5k groups: select all",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_all(),
    ));

    // A marquee over half the map: what each frame of the drag recomputes.
    let half: Vec<NodeId> = document
        .selectable_brushes()
        .into_iter()
        .map(|(id, _)| id)
        .step_by(2)
        .collect();
    rows.push(bench::measure_on(
        &format!(
            "5k groups: marquee catching {} brushes (one frame)",
            half.len()
        ),
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| {
            let empty = d.selection_snapshot();
            d.preview_marquee(&empty, &half, false);
            d.selection_count()
        },
    ));
    let small = member_of(&document, bench::FIRST_SMALL_GROUP_ID + 2_500);
    rows.push(bench::measure_on(
        "5k groups: click a small group (select_only)",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_only(small),
    ));
    let big = member_of(&document, bench::BIG_GROUP_ID);
    rows.push(bench::measure_on(
        "5k groups: click the 5k-brush group (select_only)",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_only(big),
    ));
    let deep = deepest_member(&document, &spec);
    rows.push(bench::measure_on(
        "5k groups: click a brush 10 groups deep",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_only(deep),
    ));
    rows.push(bench::measure_on(
        "5k groups: open the outermost of 10 nested groups",
        runs,
        &mut document,
        |d| {
            d.close_group();
            d.select_only(deep);
        },
        |d| d.open_selected_group(),
    ));
    rows.push(bench::measure_on(
        "5k groups: close it",
        runs,
        &mut document,
        |d| {
            d.select_only(deep);
            d.open_selected_group();
        },
        |d| d.close_group(),
    ));
    document.close_group();

    // The whole big group, moved and turned as one.
    document.select_only(big);
    assert_eq!(document.selection_count(), spec.big_group);
    rows.push(bench::measure(
        "5k groups: move the 5k-brush group, 60-step drag",
        runs.min(3),
        || (),
        |_| {
            document.begin_transaction();
            for _ in 0..60 {
                assert!(document.translate_selection(Vec3d::new(8.0, 0.0, 0.0)));
            }
            document.commit_transaction();
        },
    ));
    let centre = document.selection_bounds().unwrap().center();
    rows.push(bench::measure(
        "5k groups: rotate the 5k-brush group, 60-step drag",
        runs.min(3),
        || (),
        |_| {
            document.begin_transaction();
            for step in 1..=60 {
                document.rollback_transaction();
                assert!(document.rotate_selection_about(
                    centre,
                    Vec3d::new(0.0, 0.0, 1.0),
                    step as f64 * 1.7
                ));
            }
            document.commit_transaction();
        },
    ));
    rows.push(bench::measure_on(
        "5k groups: undo that (redo between runs)",
        runs,
        &mut document,
        |d| {
            d.redo();
        },
        |d| d.undo(),
    ));

    // Grouping and ungrouping a big selection of loose brushes.
    let loose: Vec<NodeId> = document
        .selectable_brushes()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| {
            map_model::node::outermost_group_of(document.tree_world(), *id, None).is_none()
        })
        .collect();
    eprintln!("{} loose brushes", loose.len());
    rows.push(bench::measure_on(
        &format!("5k groups: group {} loose brushes", loose.len()),
        runs,
        &mut document,
        |d| {
            d.clear_selection();
            for id in &loose {
                d.toggle_selected(*id);
            }
        },
        |d| {
            let g = d.group_selection("big new group");
            d.undo();
            g
        },
    ));
    rows.push(bench::measure_on(
        "5k groups: ungroup the 5k-brush group",
        runs,
        &mut document,
        |d| d.select_only(big),
        |d| {
            let n = d.ungroup_selection();
            d.undo();
            n
        },
    ));
    // Every small group selected and ungrouped at once.
    rows.push(bench::measure_on(
        "5k groups: ungroup 5,000 selected groups",
        runs.min(3),
        &mut document,
        |d| {
            d.clear_selection();
            for i in 0..spec.small_groups as u64 {
                let id = member_of(d, bench::FIRST_SMALL_GROUP_ID + i);
                d.toggle_selected(id);
            }
        },
        |d| {
            let n = d.ungroup_selection();
            d.undo();
            n
        },
    ));
    bench::print_table(
        "Groups (5k groups, 10-deep nesting, a 5k-brush group)",
        &rows,
    );
}

#[test]
#[ignore]
fn perf_linked_groups() {
    let scratch = Scratch::new("linked");
    let mut rows: Vec<Stats> = Vec::new();
    let runs = runs();
    let path = write(&scratch.0, "linked", bench::linked_map(500, 50, 1_000));
    let mut document = open(&path);
    eprintln!(
        "linked map: {} brushes, {} groups",
        document.brush_count(),
        document.groups().len()
    );

    // One copy opened, one brush in it moved: the edit carries the other 499 along.
    let copy = member_of(&document, 10_000 + 250);
    document.select_only(copy);
    assert!(document.open_selected_group());
    let first =
        map_model::node::contents_of(document.tree_world(), group_node(&document, 10_250))[0];
    document.select_only(first);
    assert_eq!(document.selection_count(), 1);
    let count = document.brush_count();
    rows.push(bench::measure_on(
        "500 linked x 50: move one brush in one copy (edit + sync)",
        runs,
        &mut document,
        |d| {
            let first = map_model::node::contents_of(d.tree_world(), group_node(d, 10_250))[0];
            d.select_only(first);
        },
        |d| d.translate_selection(Vec3d::new(8.0, 0.0, 0.0)),
    ));
    assert_eq!(
        document.brush_count(),
        count,
        "the sync rebuilt the copies, it did not add to them"
    );
    rows.push(bench::measure_on(
        "500 linked x 50: 30-step drag in one copy, then commit (sync)",
        runs.min(3),
        &mut document,
        |d| {
            let first = map_model::node::contents_of(d.tree_world(), group_node(d, 10_250))[0];
            d.select_only(first);
        },
        |d| {
            d.begin_transaction();
            for _ in 0..30 {
                d.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
            }
            d.commit_transaction()
        },
    ));
    rows.push(bench::measure_on(
        "500 linked x 50: undo the synced edit (redo between runs)",
        runs,
        &mut document,
        |d| {
            d.redo();
        },
        |d| d.undo(),
    ));
    document.close_group();
    rows.push(bench::measure_on(
        "500 linked x 50: sync_linked_groups from one copy",
        runs,
        &mut document,
        |d| {
            let m = member_of(d, 10_000 + 7);
            d.select_only(m);
        },
        |d| d.sync_linked_groups(),
    ));
    rows.push(bench::measure_on(
        "500 linked x 50: create a linked duplicate",
        runs,
        &mut document,
        |d| {
            let m = member_of(d, 10_000 + 3);
            d.select_only(m);
        },
        |d| {
            let r = d.create_linked_duplicate(64.0);
            d.undo();
            r
        },
    ));
    rows.push(bench::measure_on(
        "500 linked x 50: move a whole copy (60 steps)",
        runs.min(3),
        &mut document,
        |d| {
            let m = member_of(d, 10_000 + 9);
            d.select_only(m);
        },
        |d| {
            d.begin_transaction();
            for _ in 0..60 {
                d.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
            }
            d.commit_transaction()
        },
    ));
    rows.push(bench::measure_on(
        "500 linked x 50: select all",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_all(),
    ));
    bench::print_table("Linked groups (500 copies of a 50-brush group)", &rows);
}

#[test]
#[ignore]
fn perf_big_transforms() {
    let scratch = Scratch::new("transforms");
    let mut rows: Vec<Stats> = Vec::new();
    let filters = ViewFilters::default();
    let path = write(
        &scratch.0,
        "m50k",
        bench::synthetic_map(&MapSpec::brushes(50_000)),
    );
    // A world four times as wide as the map, so turning the whole map keeps it inside.
    let mut document = Document::open(&path, WorldBounds::new(65_536.0)).unwrap();
    for selected in [10_000usize, 50_000] {
        if selected >= 50_000 {
            document.select_all();
        } else {
            select_middle(&mut document, selected);
        }
        let selected = document.selection_count();
        let runs = if selected > 20_000 { 3 } else { runs().min(5) };
        let centre = document.selection_bounds().unwrap().center();
        for lock in [false, true] {
            set_alignment_lock(lock);
            let l = if lock { "lock on" } else { "lock off" };
            rows.push(bench::measure(
                &format!("50k map, {selected} selected: rotate 37° once ({l})"),
                runs,
                || (),
                |_| {
                    assert!(document.rotate_selection_about(
                        centre,
                        Vec3d::new(0.0, 0.0, 1.0),
                        37.0
                    ));
                    document.undo();
                },
            ));
            rows.push(bench::measure(
                &format!("50k map, {selected} selected: rotate drag, 30 steps of 1.7° ({l})"),
                runs.min(3),
                || (),
                |_| {
                    document.begin_transaction();
                    for step in 1..=30 {
                        document.rollback_transaction();
                        assert!(document.rotate_selection_about(
                            centre,
                            Vec3d::new(0.3, 0.2, 1.0),
                            step as f64 * 1.7
                        ));
                    }
                    document.commit_transaction();
                    document.undo();
                },
            ));
        }
        set_alignment_lock(false);
        let start = document.selection_bounds().unwrap();
        rows.push(bench::measure(
            &format!("50k map, {selected} selected: scale drag, 30 steps"),
            runs.min(3),
            || (),
            |_| {
                document.begin_transaction();
                for step in 1..=30 {
                    document.rollback_transaction();
                    let grown = Bbox3d::new(
                        start.min,
                        start.max + Vec3d::new(step as f64 * 40.0, step as f64 * 20.0, 0.0),
                    );
                    assert!(document.transform_selection(
                        &geometry_math::mat_ext::scale_bbox_matrix(&start, &grown)
                    ));
                }
                document.commit_transaction();
                document.undo();
            },
        ));
        rows.push(bench::measure(
            &format!("50k map, {selected} selected: flip"),
            runs,
            || (),
            |_| {
                assert!(document.flip_selection(Axis::X));
                document.undo();
            },
        ));
        // What a frame of the live drag pays on top of the model: the flat views' edge
        // lists and the 3D surfaces.
        for (what, mesh) in [("flat-view edges", false), ("edges + 3D surfaces", true)] {
            rows.push(bench::measure(
                &format!("50k map, {selected} selected: rotate drag + {what}, 10 steps"),
                runs.min(3),
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
                        std::hint::black_box(document.edges().len());
                        std::hint::black_box(document.selected_edges().len());
                        if mesh {
                            std::hint::black_box(document.preview_mesh(&filters).triangles.len());
                        }
                    }
                    document.commit_transaction();
                    document.undo();
                },
            ));
        }
    }

    // Handle drags over a thousand selected brushes: one vertex, one edge (two vertices)
    // or the top face (every top vertex) of each.
    select_middle(&mut document, 1_000);
    let tops: Vec<Vec<Vec3d>> = document
        .selected_brushes()
        .iter()
        .map(|b| {
            let top = b.bounds().max.z();
            let mut v: Vec<Vec3d> = b
                .vertex_positions()
                .into_iter()
                .filter(|v| (v.z() - top).abs() < 1e-6)
                .collect();
            v.sort_by(|a, b| a.x().total_cmp(&b.x()).then(a.y().total_cmp(&b.y())));
            v
        })
        .collect();
    for (what, take) in [("vertex", 1usize), ("edge", 2), ("face", usize::MAX)] {
        let handles: Vec<Vec3d> = tops
            .iter()
            .flat_map(|v| v.iter().take(take).copied())
            .collect();
        rows.push(bench::measure(
            &format!(
                "50k map, 1,000 selected: {what} drag ({} handles, 20 steps)",
                handles.len()
            ),
            runs().min(3),
            || (),
            |_| {
                document.begin_transaction();
                let mut at = handles.clone();
                for _ in 0..20 {
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

    // Clip 5,000 brushes; subtract 5 and 50 from the rest.
    select_middle(&mut document, 5_000);
    let c = document.selection_bounds().unwrap().center();
    let mut clip = editor_core::clip_tool::ClipTool::new();
    clip.set_face([
        c,
        c + Vec3d::new(0.0, 1.0, 0.0),
        c + Vec3d::new(0.3, 0.0, 1.0),
    ]);
    rows.push(bench::measure_on(
        "50k map: clip 5,000 selected",
        runs().min(3),
        &mut document,
        |d| select_middle(d, 5_000),
        |d| {
            let r = d.run_as_one_edit(|d| d.clip_selection(&clip));
            d.undo();
            r
        },
    ));
    for tools in [5usize, 50] {
        rows.push(bench::measure_on(
            &format!("50k map: subtract {tools} selected from the rest"),
            runs().min(3),
            &mut document,
            |d| select_middle(d, tools),
            |d| {
                let r = d.run_as_one_edit(|d| d.subtract_selection());
                d.undo();
                r
            },
        ));
    }
    bench::print_table("Big transforms and tools on a 50k-brush map", &rows);
}

#[test]
#[ignore]
fn perf_entities_layers_big_brush() {
    let scratch = Scratch::new("entities");
    let mut rows: Vec<Stats> = Vec::new();
    let runs = runs();
    let filters = ViewFilters::default();

    // 20,000 point entities over 5,000 brushes.
    let path = write(
        &scratch.0,
        "entities",
        bench::point_entity_map(20_000, 5_000),
    );
    rows.push(bench::measure(
        "20k entities: open",
        runs.min(3),
        || (),
        |_| open(&path),
    ));
    let mut document = open(&path);
    rows.push(bench::measure_on(
        "20k entities: mark_changed (any edit's bookkeeping)",
        runs,
        &mut document,
        |_| (),
        |d| d.mark_changed(),
    ));
    rows.push(bench::measure_on(
        "20k entities: select all",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_all(),
    ));
    let entities: Vec<NodeId> = map_model::node::point_entities(document.tree_world())
        .into_iter()
        .map(|(id, _)| id)
        .take(1_000)
        .collect();
    rows.push(bench::measure_on(
        "20k entities: move 1,000 entities, 30-step drag",
        runs.min(3),
        &mut document,
        |d| {
            d.clear_selection();
            for id in &entities {
                d.toggle_selected(*id);
            }
        },
        |d| {
            d.begin_transaction();
            for _ in 0..30 {
                d.translate_selection(Vec3d::new(8.0, 0.0, 0.0));
            }
            d.commit_transaction();
        },
    ));
    rows.push(bench::measure_on(
        "20k entities: 3D surfaces (preview_mesh, cold)",
        runs,
        &mut document,
        |d| d.mark_changed(),
        |d| d.preview_mesh(&filters).triangles.len(),
    ));
    rows.push(bench::measure(
        "20k entities: validation",
        runs.min(3),
        || (),
        |_| editor_core::validation::validate(&document).len(),
    ));
    drop(document);

    // 200 layers over 20,000 brushes.
    let path = write(&scratch.0, "layers", bench::layered_map(200, 20_000));
    let mut document = open(&path);
    eprintln!("{} layers", document.layers().len());
    rows.push(bench::measure(
        "200 layers: layers() (the Map tab's list)",
        runs,
        || (),
        |_| document.layers().len(),
    ));
    rows.push(bench::measure_on(
        "200 layers: hide and show a layer",
        runs,
        &mut document,
        |_| (),
        |d| {
            d.set_layer_hidden(100, true);
            d.set_layer_hidden(100, false);
        },
    ));
    rows.push(bench::measure_on(
        "200 layers: select a layer",
        runs,
        &mut document,
        |d| d.clear_selection(),
        |d| d.select_layer(150),
    ));
    rows.push(bench::measure_on(
        "200 layers: flat view edges (cold)",
        runs,
        &mut document,
        |d| d.mark_changed(),
        |d| d.edges().len(),
    ));
    drop(document);

    // A 512-sided cylinder: its top ring dragged.
    let mut document = Document::new(WorldBounds::default());
    let big = map_model::shapes::create_cylinder(
        &document.world_bounds(),
        &Bbox3d::new(Vec3d::new(0.0, 0.0, 0.0), Vec3d::new(4096.0, 4096.0, 512.0)),
        map_model::shapes::CircleShape::EdgeAligned { num_sides: 512 },
        Axis::Z,
        "bench/m00001",
    )
    .unwrap();
    let top: Vec<Vec3d> = big
        .vertex_positions()
        .into_iter()
        .filter(|v| v.z() > 500.0)
        .collect();
    let one: Vec<Vec3d> = top.iter().take(1).copied().collect();
    document.add_brushes(vec![big]);
    eprintln!("512-sided cylinder: {} top vertices", top.len());
    for (what, handles) in [("one vertex", one), ("the whole top ring", top)] {
        rows.push(bench::measure(
            &format!("512-sided cylinder: drag {what}, 20 steps"),
            runs.min(3),
            || (),
            |_| {
                document.begin_transaction();
                let mut at = handles.clone();
                for _ in 0..20 {
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
    rows.push(bench::measure_on(
        "512-sided cylinder: 3D surfaces (cold)",
        runs,
        &mut document,
        |d| d.mark_changed(),
        |d| d.preview_mesh(&filters).triangles.len(),
    ));
    bench::print_table("Many entities, many layers, one huge brush", &rows);
}

/// Undo history: 500 edits on a 50k-brush map, each moving 100 brushes; how long each
/// takes and how much the process grows.
#[test]
#[ignore]
fn perf_history_growth() {
    let scratch = Scratch::new("history");
    let path = write(
        &scratch.0,
        "m50k",
        bench::synthetic_map(&MapSpec::brushes(50_000)),
    );
    let mut document = open(&path);
    select_middle(&mut document, 100);
    let before = bench::current_rss();
    let start = Instant::now();
    let mut times = Vec::new();
    for i in 0..500 {
        let t = Instant::now();
        document.translate_selection(Vec3d::new(if i % 2 == 0 { 8.0 } else { -8.0 }, 0.0, 0.0));
        times.push(t.elapsed());
    }
    let total = start.elapsed();
    let after = bench::current_rss();
    times.sort();
    println!(
        "\n500 edits of 100 brushes on 50k: {:.0} ms total, median {:.3} ms, p95 {:.3} ms; RSS {:.0} -> {:.0} MB (+{:.1} MB, {:.0} KB per edit)",
        bench::ms(total),
        bench::ms(times[250]),
        bench::ms(times[475]),
        before as f64 / 1e6,
        after as f64 / 1e6,
        (after as f64 - before as f64) / 1e6,
        (after as f64 - before as f64) / 1e3 / 500.0
    );
    let t = Instant::now();
    for _ in 0..500 {
        document.undo();
    }
    println!("undo all 500: {:.0} ms", bench::ms(t.elapsed()));
}

/// Adding a small folder of textures after a big one: the big one is not decoded again.
#[test]
#[ignore]
fn perf_material_folders() {
    let count: usize = std::env::var("HERON_BENCH_TEXTURES")
        .ok()
        .and_then(|s| s.split(',').next()?.trim().parse().ok())
        .unwrap_or(2_000);
    let big = Scratch::new("folder-big");
    bench::write_textures(&big.0, count, 0, 3).unwrap();
    let small = Scratch::new("folder-small");
    bench::write_textures(&small.0, 10, 0, 9).unwrap();
    // Names of their own, or they would replace ten of the big folder's.
    std::fs::rename(small.0.join("bench"), small.0.join("extra")).unwrap();
    let mut rows: Vec<Stats> = Vec::new();
    rows.push(bench::measure(
        &format!("{count} textures: load the folder"),
        1,
        || Document::new(WorldBounds::default()),
        |mut document| {
            document.load_materials(&big.0).unwrap();
            document
        },
    ));
    let mut document = Document::new(WorldBounds::default());
    document.load_materials(&big.0).unwrap();
    rows.push(bench::measure_on(
        &format!("{count} textures loaded: add a folder of 10"),
        3,
        &mut document,
        |_| (),
        |d| d.load_materials(&small.0).unwrap(),
    ));
    rows.push(bench::measure_on(
        &format!("{count} + 10 textures: take the small folder away"),
        3,
        &mut document,
        |d| {
            d.load_materials(&small.0).unwrap();
        },
        |d| d.remove_material_source(1),
    ));
    bench::print_table("Material folders", &rows);
}
